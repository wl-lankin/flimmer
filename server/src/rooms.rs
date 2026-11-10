//! Pure room logic: no sockets, no clocks (time is passed in), fully unit-testable.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rand::Rng;

pub type ConnId = u64;

pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
pub const CODE_LEN: usize = 6;
pub const MAX_MEMBERS: usize = 8;
pub const MAX_NAME_CHARS: usize = 32;
pub const DEFAULT_NAME: &str = "Gast";

/// Trim, truncate to 32 chars (never splitting a char), fall back to "Gast".
pub fn sanitize_name(raw: &str) -> String {
    let name: String = raw
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        DEFAULT_NAME.to_string()
    } else {
        name.to_string()
    }
}

pub fn generate_code<R: Rng>(rng: &mut R) -> String {
    (0..CODE_LEN)
        .map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub id: ConnId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinError {
    NotFound,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    pub room: String,
    pub remaining: Vec<ConnId>,
    pub deleted: bool,
}

#[derive(Default)]
pub struct Hub {
    rooms: HashMap<String, Vec<Member>>,
    membership: HashMap<ConnId, String>,
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn room_of(&self, conn: ConnId) -> Option<&str> {
        self.membership.get(&conn).map(String::as_str)
    }

    /// Creates a room with a fresh unique code. Also returns the departure from
    /// the previous room, if any, so the caller can notify its members.
    pub fn create<R: Rng>(
        &mut self,
        conn: ConnId,
        name: &str,
        rng: &mut R,
    ) -> (String, Option<Departure>) {
        let departure = self.leave(conn);
        let code = loop {
            let c = generate_code(rng);
            if !self.rooms.contains_key(&c) {
                break c;
            }
        };
        self.rooms.insert(
            code.clone(),
            vec![Member {
                id: conn,
                name: sanitize_name(name),
            }],
        );
        self.membership.insert(conn, code.clone());
        (code, departure)
    }

    /// Returns the normalized (upper-case) code and the departure from the
    /// previous room. A failed join leaves the current membership untouched.
    pub fn join(
        &mut self,
        conn: ConnId,
        code: &str,
        name: &str,
    ) -> Result<(String, Option<Departure>), JoinError> {
        let code = code.trim().to_ascii_uppercase();
        if self.membership.get(&conn) == Some(&code) {
            return Ok((code, None));
        }
        let len = self.rooms.get(&code).ok_or(JoinError::NotFound)?.len();
        if len >= MAX_MEMBERS {
            return Err(JoinError::Full);
        }
        let departure = self.leave(conn);
        self.rooms
            .get_mut(&code)
            .ok_or(JoinError::NotFound)?
            .push(Member {
                id: conn,
                name: sanitize_name(name),
            });
        self.membership.insert(conn, code.clone());
        Ok((code, departure))
    }

    pub fn leave(&mut self, conn: ConnId) -> Option<Departure> {
        let room = self.membership.remove(&conn)?;
        let members = self.rooms.get_mut(&room)?;
        members.retain(|m| m.id != conn);
        let remaining: Vec<ConnId> = members.iter().map(|m| m.id).collect();
        let deleted = remaining.is_empty();
        if deleted {
            self.rooms.remove(&room);
        }
        Some(Departure {
            room,
            remaining,
            deleted,
        })
    }

    pub fn members(&self, room: &str) -> &[Member] {
        self.rooms.get(room).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn member_ids(&self, room: &str) -> Vec<ConnId> {
        self.members(room).iter().map(|m| m.id).collect()
    }

    /// Everyone in the sender's room except the sender.
    pub fn others(&self, conn: ConnId) -> Vec<ConnId> {
        match self.room_of(conn) {
            Some(room) => self
                .members(room)
                .iter()
                .map(|m| m.id)
                .filter(|id| *id != conn)
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn name_of(&self, conn: ConnId) -> Option<&str> {
        let room = self.room_of(conn)?;
        self.members(room)
            .iter()
            .find(|m| m.id == conn)
            .map(|m| m.name.as_str())
    }
}

/// Fixed-window limiter: at most `limit` messages per `window`.
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    window_start: Option<Instant>,
    count: u32,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            window_start: None,
            count: 0,
        }
    }

    pub fn allow(&mut self, now: Instant) -> bool {
        match self.window_start {
            Some(start) if now.duration_since(start) < self.window => {}
            _ => {
                self.window_start = Some(now);
                self.count = 0;
            }
        }
        if self.count < self.limit {
            self.count += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    #[test]
    fn name_sanitizing() {
        assert_eq!(sanitize_name("  Anna  "), "Anna");
        assert_eq!(sanitize_name(""), "Gast");
        assert_eq!(sanitize_name("   "), "Gast");
        assert_eq!(sanitize_name(&"a".repeat(40)).chars().count(), 32);
        let umlauts = "ä".repeat(40);
        let cut = sanitize_name(&umlauts);
        assert_eq!(cut.chars().count(), 32);
        assert!(cut.chars().all(|c| c == 'ä'));
        assert_eq!(sanitize_name("a\nb"), "ab");
    }

    #[test]
    fn codes_use_alphabet() {
        let mut r = rng();
        for _ in 0..200 {
            let c = generate_code(&mut r);
            assert_eq!(c.len(), CODE_LEN);
            assert!(c.bytes().all(|b| CODE_ALPHABET.contains(&b)));
            assert!(!c.contains(['0', 'O', '1', 'I']));
        }
    }

    #[test]
    fn create_join_leave() {
        let mut hub = Hub::new();
        let mut r = rng();
        let (code, _) = hub.create(1, "A", &mut r);
        assert_eq!(hub.room_of(1), Some(code.as_str()));
        assert_eq!(hub.join(2, &code.to_lowercase(), "B").unwrap().0, code);
        assert_eq!(hub.others(1), vec![2]);
        assert_eq!(hub.others(2), vec![1]);
        assert_eq!(hub.name_of(2), Some("B"));
        let dep = hub.leave(1).unwrap();
        assert_eq!(dep.remaining, vec![2]);
        assert!(!dep.deleted);
        let dep = hub.leave(2).unwrap();
        assert!(dep.deleted);
        assert_eq!(hub.room_count(), 0);
        assert!(hub.leave(2).is_none());
    }

    #[test]
    fn join_errors_and_capacity() {
        let mut hub = Hub::new();
        let mut r = rng();
        assert_eq!(hub.join(1, "ZZZZZZ", "x"), Err(JoinError::NotFound));
        let (code, _) = hub.create(1, "host", &mut r);
        for id in 2..=MAX_MEMBERS as u64 {
            assert!(hub.join(id, &code, "m").is_ok());
        }
        assert_eq!(hub.join(99, &code, "late"), Err(JoinError::Full));
        assert_eq!(hub.room_of(99), None);
        // A failed join keeps the old room.
        let (other, _) = hub.create(50, "o", &mut r);
        assert_eq!(hub.join(50, &code, "o"), Err(JoinError::Full));
        assert_eq!(hub.room_of(50), Some(other.as_str()));
    }

    #[test]
    fn switching_rooms_leaves_old_one() {
        let mut hub = Hub::new();
        let mut r = rng();
        let (a, _) = hub.create(1, "a", &mut r);
        let (b, _) = hub.create(2, "b", &mut r);
        assert_ne!(a, b);
        let (_, dep) = hub.join(1, &b, "a").unwrap();
        assert!(dep.unwrap().deleted);
        assert_eq!(hub.room_count(), 1);
        assert_eq!(hub.member_ids(&b).len(), 2);
    }

    #[test]
    fn rejoining_own_room_is_noop() {
        let mut hub = Hub::new();
        let mut r = rng();
        let (a, _) = hub.create(1, "a", &mut r);
        assert_eq!(hub.join(1, &a, "a"), Ok((a, None)));
        assert_eq!(hub.room_count(), 1);
    }

    #[test]
    fn rate_limiter_window() {
        let mut rl = RateLimiter::new(3, Duration::from_secs(1));
        let t0 = Instant::now();
        assert!(rl.allow(t0));
        assert!(rl.allow(t0));
        assert!(rl.allow(t0));
        assert!(!rl.allow(t0));
        assert!(!rl.allow(t0 + Duration::from_millis(999)));
        assert!(rl.allow(t0 + Duration::from_millis(1000)));
    }
}

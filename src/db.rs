use std::path::PathBuf;

use rusqlite::{Connection, Result, params};
use rusqlite::types::Value;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = db_path();
        std::fs::create_dir_all(path.parent().unwrap()).expect("failed to create data dir");
        let conn = Connection::open(&path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch("
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS keystrokes (
                id        INTEGER PRIMARY KEY,
                ts        INTEGER NOT NULL,
                key       TEXT    NOT NULL,
                modifiers TEXT    NOT NULL,
                app_class TEXT    NOT NULL,
                app_title TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_app ON keystrokes(app_class);
            CREATE TABLE IF NOT EXISTS bigrams (
                id        INTEGER PRIMARY KEY,
                ts        INTEGER NOT NULL,
                prev_key  TEXT    NOT NULL,
                prev_mods TEXT    NOT NULL,
                curr_key  TEXT    NOT NULL,
                curr_mods TEXT    NOT NULL,
                app_class TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bigrams_app  ON bigrams(app_class);
            CREATE INDEX IF NOT EXISTS idx_bigrams_prev ON bigrams(prev_key);
            CREATE INDEX IF NOT EXISTS idx_bigrams_curr ON bigrams(curr_key);
        ")?;
        Ok(Self { conn })
    }

    pub fn insert(&self, ts: i64, key: &str, modifiers: &str, app_class: &str, app_title: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO keystrokes (ts, key, modifiers, app_class, app_title)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ts, key, modifiers, app_class, app_title],
        )?;
        Ok(())
    }

    pub fn insert_bigram(&self, ts: i64, prev_key: &str, prev_mods: &str, curr_key: &str, curr_mods: &str, app_class: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO bigrams (ts, prev_key, prev_mods, curr_key, curr_mods, app_class)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ts, prev_key, prev_mods, curr_key, curr_mods, app_class],
        )?;
        Ok(())
    }

    pub fn top_bigrams(&self, apps: &[String], limit: Option<usize>) -> Result<Vec<(String, String, String, String, String, i64)>> {
        let limit = limit.map(|n| n as i64).unwrap_or(-1);

        if apps.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT prev_key, prev_mods, curr_key, curr_mods, app_class, COUNT(*) FROM bigrams
                 GROUP BY prev_key, prev_mods, curr_key, curr_mods, app_class ORDER BY COUNT(*) DESC LIMIT ?1",
            )?;
            stmt.query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
            })?.collect()
        } else {
            let placeholders = (1..=apps.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
            let sql = format!(
                "SELECT prev_key, prev_mods, curr_key, curr_mods, app_class, COUNT(*) FROM bigrams
                 WHERE app_class IN ({placeholders})
                 GROUP BY prev_key, prev_mods, curr_key, curr_mods, app_class ORDER BY COUNT(*) DESC LIMIT ?{}",
                apps.len() + 1
            );
            let mut params: Vec<Value> = apps.iter().map(|s| Value::Text(s.clone())).collect();
            params.push(Value::Integer(limit));
            let mut stmt = self.conn.prepare(&sql)?;
            stmt.query_map(rusqlite::params_from_iter(params), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
            })?.collect()
        }
    }

    pub fn top_keys_global(&self, limit: Option<usize>) -> Result<Vec<(String, String, i64)>> {
        let limit = limit.map(|n| n as i64).unwrap_or(-1);
        let mut stmt = self.conn.prepare(
            "SELECT key, modifiers, COUNT(*) FROM keystrokes
             GROUP BY key, modifiers ORDER BY COUNT(*) DESC LIMIT ?1",
        )?;
        stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect()
    }

    pub fn top_keys(&self, apps: &[String], limit: Option<usize>) -> Result<Vec<(String, String, String, i64)>> {
        let limit = limit.map(|n| n as i64).unwrap_or(-1);

        if apps.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT key, modifiers, app_class, COUNT(*) FROM keystrokes
                 GROUP BY key, modifiers, app_class ORDER BY COUNT(*) DESC LIMIT ?1",
            )?;
            stmt.query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?.collect()
        } else {
            let placeholders = (1..=apps.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
            let sql = format!(
                "SELECT key, modifiers, app_class, COUNT(*) FROM keystrokes
                 WHERE app_class IN ({placeholders})
                 GROUP BY key, modifiers, app_class ORDER BY COUNT(*) DESC LIMIT ?{}",
                apps.len() + 1
            );
            let mut params: Vec<Value> = apps.iter().map(|s| Value::Text(s.clone())).collect();
            params.push(Value::Integer(limit));
            let mut stmt = self.conn.prepare(&sql)?;
            stmt.query_map(rusqlite::params_from_iter(params), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?.collect()
        }
    }

    pub fn top_apps(&self, limit: Option<usize>) -> Result<Vec<(String, i64)>> {
        let limit = limit.map(|n| n as i64).unwrap_or(-1);
        let mut stmt = self.conn.prepare(
            "SELECT app_class, COUNT(*) FROM keystrokes
             WHERE app_class != ''
             GROUP BY app_class ORDER BY COUNT(*) DESC LIMIT ?1",
        )?;
        stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect()
    }

    pub fn clear(&self, app_class: Option<&str>) -> Result<usize> {
        match app_class {
            Some(app) => {
                self.conn.execute("DELETE FROM bigrams WHERE app_class = ?1", params![app])?;
                self.conn.execute("DELETE FROM keystrokes WHERE app_class = ?1", params![app])
            }
            None => {
                self.conn.execute("DELETE FROM bigrams", [])?;
                self.conn.execute("DELETE FROM keystrokes", [])
            }
        }
    }
}

fn db_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").expect("HOME not set")).join(".local/share")
        });
    base.join("regkey/keystrokes.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE keystrokes (
                id        INTEGER PRIMARY KEY,
                ts        INTEGER NOT NULL,
                key       TEXT    NOT NULL,
                modifiers TEXT    NOT NULL,
                app_class TEXT    NOT NULL,
                app_title TEXT    NOT NULL
            );
            CREATE INDEX idx_app ON keystrokes(app_class);
            CREATE TABLE bigrams (
                id        INTEGER PRIMARY KEY,
                ts        INTEGER NOT NULL,
                prev_key  TEXT    NOT NULL,
                prev_mods TEXT    NOT NULL,
                curr_key  TEXT    NOT NULL,
                curr_mods TEXT    NOT NULL,
                app_class TEXT    NOT NULL
            );
            CREATE INDEX idx_bigrams_app  ON bigrams(app_class);
            CREATE INDEX idx_bigrams_prev ON bigrams(prev_key);
            CREATE INDEX idx_bigrams_curr ON bigrams(curr_key);
        ").unwrap();
        Db { conn }
    }

    #[test]
    fn insert_and_top_keys_global() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "a", "", "kitty", "").unwrap();
        db.insert(3, "b", "", "emacs", "").unwrap();

        let rows = db.top_keys(&[], None).unwrap();
        assert_eq!(rows[0], ("a".into(), "".into(), "kitty".into(), 2));
        assert_eq!(rows[1], ("b".into(), "".into(), "emacs".into(), 1));
    }

    #[test]
    fn top_keys_filter_single_app() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "b", "", "emacs", "").unwrap();
        db.insert(3, "b", "", "emacs", "").unwrap();

        let rows = db.top_keys(&["emacs".to_string()], None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "b");
        assert_eq!(rows[0].3, 2);
    }

    #[test]
    fn top_keys_union() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "b", "", "emacs", "").unwrap();
        db.insert(3, "c", "", "firefox", "").unwrap();

        let apps = vec!["kitty".to_string(), "emacs".to_string()];
        let rows = db.top_keys(&apps, None).unwrap();
        let keys: Vec<&str> = rows.iter().map(|(k, _, _, _)| k.as_str()).collect();
        assert!(keys.contains(&"a"));
        assert!(keys.contains(&"b"));
        assert!(!keys.contains(&"c"));
    }

    #[test]
    fn top_keys_with_modifiers() {
        let db = in_memory();
        db.insert(1, "a", "ctrl", "kitty", "").unwrap();
        db.insert(2, "a", "",     "kitty", "").unwrap();

        let rows = db.top_keys(&[], None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn top_keys_limit() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "z", "", "kitty", "").unwrap();
        db.insert(3, "z", "", "kitty", "").unwrap();

        let rows = db.top_keys(&[], Some(1)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "z");
    }

    #[test]
    fn top_apps() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "a", "", "kitty", "").unwrap();
        db.insert(3, "b", "", "emacs", "").unwrap();

        let apps = db.top_apps(None).unwrap();
        assert_eq!(apps[0], ("kitty".into(), 2));
        assert_eq!(apps[1], ("emacs".into(), 1));
    }

    #[test]
    fn clear_all() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "b", "", "emacs", "").unwrap();

        let deleted = db.clear(None).unwrap();
        assert_eq!(deleted, 2);
        assert!(db.top_keys(&[], None).unwrap().is_empty());
    }

    #[test]
    fn clear_by_app() {
        let db = in_memory();
        db.insert(1, "a", "", "kitty", "").unwrap();
        db.insert(2, "b", "", "emacs", "").unwrap();

        db.clear(Some("kitty")).unwrap();
        let rows = db.top_keys(&[], None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "b");
    }

    #[test]
    fn bigrams_global() {
        let db = in_memory();
        db.insert_bigram(1, "j", "", "k", "", "kitty").unwrap();
        db.insert_bigram(2, "j", "", "k", "", "kitty").unwrap();
        db.insert_bigram(3, "h", "", "j", "", "kitty").unwrap();

        let rows = db.top_bigrams(&[], None).unwrap();
        assert_eq!(rows[0], ("j".into(), "".into(), "k".into(), "".into(), "kitty".into(), 2));
        assert_eq!(rows[1], ("h".into(), "".into(), "j".into(), "".into(), "kitty".into(), 1));
    }

    #[test]
    fn bigrams_filter_app() {
        let db = in_memory();
        db.insert_bigram(1, "j", "", "k", "", "kitty").unwrap();
        db.insert_bigram(2, "a", "", "b", "", "emacs").unwrap();

        let rows = db.top_bigrams(&["emacs".to_string()], None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "a");
        assert_eq!(rows[0].2, "b");
    }

    #[test]
    fn bigrams_union() {
        let db = in_memory();
        db.insert_bigram(1, "j", "", "k", "", "kitty").unwrap();
        db.insert_bigram(2, "a", "", "b", "", "emacs").unwrap();
        db.insert_bigram(3, "x", "", "y", "", "firefox").unwrap();

        let apps = vec!["kitty".to_string(), "emacs".to_string()];
        let rows = db.top_bigrams(&apps, None).unwrap();
        let pairs: Vec<(&str, &str)> = rows.iter().map(|(p, _, c, _, _, _)| (p.as_str(), c.as_str())).collect();
        assert!(pairs.contains(&("j", "k")));
        assert!(pairs.contains(&("a", "b")));
        assert!(!pairs.contains(&("x", "y")));
    }

    #[test]
    fn bigrams_with_modifiers() {
        let db = in_memory();
        db.insert_bigram(1, "c", "ctrl", "v", "ctrl", "kitty").unwrap();
        db.insert_bigram(2, "c", "ctrl", "v", "ctrl", "kitty").unwrap();
        db.insert_bigram(3, "c", "", "v", "", "kitty").unwrap();

        let rows = db.top_bigrams(&[], None).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].5, 2);
        assert_eq!(rows[0].1, "ctrl");
    }

    #[test]
    fn clear_also_clears_bigrams() {
        let db = in_memory();
        db.insert_bigram(1, "j", "", "k", "", "kitty").unwrap();
        db.insert_bigram(2, "a", "", "b", "", "emacs").unwrap();

        db.clear(Some("kitty")).unwrap();
        assert!(db.top_bigrams(&["kitty".to_string()], None).unwrap().is_empty());
        assert_eq!(db.top_bigrams(&["emacs".to_string()], None).unwrap().len(), 1);

        db.clear(None).unwrap();
        assert!(db.top_bigrams(&[], None).unwrap().is_empty());
    }
}

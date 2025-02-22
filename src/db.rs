use rusqlite::{Connection, params, Error as RusqliteError, Result};
use std::sync::Mutex;
use std::path::Path;
use bcrypt::{hash, verify};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> Self {
        let conn = Connection::open(Path::new(db_path)).expect("Failed to open database");

        let db = Database {
            conn: Mutex::new(conn),
        };

        {
            let conn = db.conn.lock().unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS settings (
                    user_id INTEGER PRIMARY KEY,
                    min_range INTEGER NOT NULL,
                    max_range INTEGER NOT NULL,
                    max_guesses INTEGER NOT NULL,
                    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS stats (
                    user_id INTEGER PRIMARY KEY,
                    games_played INTEGER DEFAULT 0,
                    games_won INTEGER DEFAULT 0,
                    games_lost INTEGER DEFAULT 0,
                    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
                );"
            ).expect("Failed to create tables");
        }

        db
    }

    pub fn register_user(&self, username: &str, password: &str) -> Result<(), String> {
        let password_hash = hash(password, 10).map_err(|_| "Failed to hash password")?;

        let conn = self.conn.lock().unwrap();
        match conn.execute(
            "INSERT INTO users (username, password_hash) VALUES (?, ?)",
            params![username, password_hash],
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err("Username already exists".into()),
        }
    }

    pub fn authenticate_user(&self, username: &str, password: &str) -> Result<i32, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, password_hash FROM users WHERE username = ?")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query(params![username]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let user_id: i32 = row.get(0).map_err(|e: RusqliteError| e.to_string())?;
            let stored_hash: String = row.get(1).map_err(|e: RusqliteError| e.to_string())?;
            if verify(password, &stored_hash).map_err(|e| e.to_string())? {
                return Ok(user_id);
            }
        }

        Err("Invalid username or password".into())
    }
/*
    pub fn save_user_settings(&self, user_id: i32, min: i32, max: i32, guesses: i32) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO settings (user_id, min_range, max_range, max_guesses) 
             VALUES (?, ?, ?, ?) 
             ON CONFLICT(user_id) DO UPDATE SET 
             min_range=excluded.min_range, max_range=excluded.max_range, max_guesses=excluded.max_guesses",
            params![user_id, min, max, guesses],
        ).map_err(|_| "Failed to save settings")?;
        Ok(())
    }
*/

    pub fn save_user_settings(&self, user_id: i32, min: i32, max: i32, guesses: i32) -> Result<(), String> {
        let conn = self.conn.lock().unwrap(); // Ensure locked access

        let sql = "INSERT INTO settings (user_id, min_range, max_range, max_guesses)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(user_id) DO UPDATE SET
                min_range = excluded.min_range, 
                max_range = excluded.max_range, 
                max_guesses = excluded.max_guesses";

        match conn.execute(sql, params![user_id, min, max, guesses]) {
            Ok(_) => {
                log::info!("✅ Saved settings to DB - Min: {}, Max: {}, Guesses: {}", min, max, guesses);
                Ok(())
            }
            Err(e) => {
                log::error!("❌ Failed to save settings: {:?}", e);
                Err(e.to_string())
            }
        }
    }

    pub fn load_user_settings(&self, user_id: i32) -> Result<(i32, i32, i32), String> {
        let conn = self.conn.lock().unwrap(); // ✅ Lock database connection
        
        log::info!("🔍 Loading user settings for user_id: {}", user_id);

        let mut stmt = conn.prepare("SELECT min_range, max_range, max_guesses FROM settings WHERE user_id = ?")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let settings = (
                row.get(0).map_err(|e: RusqliteError| e.to_string())?,
                row.get(1).map_err(|e: RusqliteError| e.to_string())?,
                row.get(2).map_err(|e: RusqliteError| e.to_string())?,
            );
            log::info!("✅ Loaded settings from DB - Min: {}, Max: {}, Guesses: {}", settings.0, settings.1, settings.2);
            Ok(settings)
        } 
        else {
            log::warn!("⚠ No settings found for user_id: {}", user_id);
            Err("No settings found".into())
        }
    }

    pub fn update_game_stats(&self, user_id: i32, won: bool) -> Result<(), String> {
        let column = if won { "games_won" } else { "games_lost" };
        let query = format!(
            "INSERT INTO stats (user_id, games_played, {col}) 
             VALUES (?, 1, 1) 
             ON CONFLICT(user_id) DO UPDATE SET 
             games_played=stats.games_played + 1, 
             {col}=stats.{col} + 1",
            col = column
        );
    
        let conn = self.conn.lock().unwrap(); // ✅ Lock database connection

        println!("Executing SQL: {}", query);
    
        match conn.execute(&query, params![user_id]) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error updating game stats: {:?}", e);
                Err(e.to_string())
            }
        }
    }

    pub fn get_user_stats(&self, user_id: i32) -> Result<(i32, i32, i32), String> {
        let conn = self.conn.lock().unwrap(); // ✅ Lock database connection

        let mut stmt = conn.prepare("SELECT games_played, games_won, games_lost FROM stats WHERE user_id = ?")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok((
                row.get(0).map_err(|e: RusqliteError| e.to_string())?,
                row.get(1).map_err(|e: RusqliteError| e.to_string())?,
                row.get(2).map_err(|e: RusqliteError| e.to_string())?,
            ))
        } 
        else {
            Err("No stats found".into())
        }
    }   

}
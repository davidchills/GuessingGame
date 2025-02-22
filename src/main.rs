mod db;
use db::Database;
use eframe::egui;
use std::sync::Arc;
use std::sync::Mutex;
//use rand::random;
use rand::Rng;
use log::info;
use env_logger;

struct GuessingGameApp {
    db: Arc<Mutex<Database>>,
    username: String,
    password: String,
    logged_in_user_id: Option<i32>,
    min_range: i32,
    max_range: i32,
    max_guesses: i32,
    guess: String,
    message: String,
    target_number: i32,
    remaining_guesses: i32,
}

struct UserSettings {
    min_range: i32,
    max_range: i32,
    max_guesses: i32,
}

impl GuessingGameApp {
    fn reset_game(&mut self) {
        let mut rng = rand::thread_rng();
        self.target_number = rng.gen_range(self.min_range..=self.max_range);
        self.remaining_guesses = self.max_guesses;
        self.message = "♻ Game reset! Start guessing.".to_string();
        
        log::info!("🎯 New Target Number Generated: {} (Range: {}-{})", self.target_number, self.min_range, self.max_range);
    }
}

impl Default for GuessingGameApp {
    fn default() -> Self {
        log::info!("🔄 Initializing GuessingGameApp...");

        // Default values in case there's no user yet
        let mut min_range = 1;
        let mut max_range = 100;
        let mut max_guesses = 5;

        log::info!("✅ Setting default values...");

        log::info!("🔄 Creating database instance...");
        let db = Arc::new(Mutex::new(Database::new("game_data.sqlite")));

        if let Ok(db) = db.lock() {
            if let Ok(user_id) = db.authenticate_user("Dave", "securepassword") {
                log::info!("✅ User authenticated with ID: {}", user_id);
                if let Ok((min, max, guesses)) = db.load_user_settings(user_id) {
                    log::info!("✅ Loaded settings from DB - Min: {}, Max: {}, Guesses: {}", min, max, guesses);
                    min_range = min;
                    max_range = max;
                    max_guesses = guesses;
                }
                else {
                    log::warn!("⚠ Could not load user settings, using defaults.");
                }                
            }
            else {
                log::warn!("⚠ Authentication failed, using defaults.");
            }
        } 
        else {
            log::error!("Failed to lock database");
        }        

        log::info!("Min Number Set: {}", min_range);
        log::info!("Max Number Set: {}", max_range);
        log::info!("Number of Guesses Set: {}", max_guesses);

        // Generate random number based on user-defined settings
        //let target_number = rand::random_range(min_range..=max_range);
        //let mut rng = rand::thread_rng();
        //let target_number = rng.gen_range(min_range..=max_range);

        //log::info!("🎯 Target number generated: {} (Range: {}-{})", target_number, min_range, max_range);
        /* 
        Self {
            db,
            username: "Dave".to_string(),
            password: "securepassword".to_string(),
            logged_in_user_id: None,
            min_range,
            max_range,
            max_guesses,
            guess: "".to_string(),
            message: "Enter a number to start guessing!".to_string(),
            target_number: 0, // Generate a random number
            remaining_guesses: max_guesses,
        }
        */
        let mut app = Self {
            db,
            username: "Dave".to_string(),
            password: "securepassword".to_string(),
            logged_in_user_id: None,
            min_range,
            max_range,
            max_guesses,
            guess: "".to_string(),
            message: "Enter a number to start guessing!".to_string(),
            target_number: 0, // Placeholder, will be set in reset_game()
            remaining_guesses: max_guesses,
        };
        
        app.reset_game(); // ✅ Set initial random number
        app        
    }
}

impl eframe::App for GuessingGameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        //log::info!("Updating UI...");

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.heading("🎯 Guessing Game 🎯");
                ui.separator();

                if self.logged_in_user_id.is_none() {
                    ui.label("🔑 Username:");
                    ui.text_edit_singleline(&mut self.username);
                    ui.label("🔒 Password:");
                    ui.text_edit_singleline(&mut self.password);

                    if ui.button("🚀 Login").clicked() {
                        let db = self.db.lock().unwrap();
                        match db.authenticate_user(&self.username, &self.password) {
                            Ok(user_id) => {
                                let settings = db.load_user_settings(user_id).unwrap_or((1, 100, 5));
                                let temp_settings = UserSettings {
                                    min_range: settings.0,
                                    max_range: settings.1,
                                    max_guesses: settings.2,
                                };
                                // Release the database so we can free the borrowed self
                                drop(db);

                                self.logged_in_user_id = Some(user_id);
                                self.min_range = temp_settings.min_range;
                                self.max_range = temp_settings.max_range;
                                self.max_guesses = temp_settings.max_guesses;
                                self.remaining_guesses = self.max_guesses;

                                self.reset_game();
                                
                                self.message = format!("🎉 Welcome, {}! Start guessing!", self.username);
                            }
                            Err(_) => self.message = "❌ Invalid username or password.".to_string(),
                        }
                    }
                    ui.label(&self.message);
                    return;
                }

                ui.label(format!("👤 Playing as {}", self.username));
                ui.label(format!("🎲 Guess a number between {} and {}.", self.min_range, self.max_range));
                ui.add(egui::TextEdit::singleline(&mut self.guess).hint_text("Enter your guess"));
                ui.label(format!("You have {} guesses left.", self.remaining_guesses));
                if ui.button("✅ Submit Guess").clicked() {
                    if let Ok(num) = self.guess.parse::<i32>() {
                        self.remaining_guesses -= 1;
                        if num == self.target_number {
                            self.message = "🎉 You guessed it! You win!".to_string();
                            let db = self.db.lock().unwrap();
                            db.update_game_stats(self.logged_in_user_id.unwrap(), true).unwrap();
                        } 
                        else if self.remaining_guesses == 0 {
                            self.message = format!("😢 You lost! The number was {}.", self.target_number);
                            let db = self.db.lock().unwrap();
                            db.update_game_stats(self.logged_in_user_id.unwrap(), false).unwrap();
                        } 
                        else if num < self.target_number {
                            self.message = "⬆ Too low! Try again.".to_string();
                        } 
                        else {
                            self.message = "⬇ Too high! Try again.".to_string();
                        }
                    } 
                    else {
                        self.message = "⚠ Please enter a valid number!".to_string();
                    }
                }

                ui.label(&self.message);

                if ui.button("🔄 Reset Game").clicked() {
                    self.reset_game();
                }

                if ui.button("🚪 Logout").clicked() {
                    self.logged_in_user_id = None;
                    self.username.clear();
                    self.password.clear();
                    self.message = "🔑 Enter your username and password.".to_string();
                }
            });
        });
    }
}


 
fn main() -> Result<(), eframe::Error> {

    env_logger::init();
    info!("Starting Guessing Game...");

    let db = Database::new("game_data.sqlite");

    // Attempt to register user, but handle "Username already exists" gracefully
    match db.register_user("Dave", "securepassword") {
        Ok(_) => println!("User registered successfully."),
        Err(e) if e == "Username already exists" => println!("User already exists, skipping registration."),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // Authenticate User
    let user_id = db.authenticate_user("Dave", "securepassword")
        .expect("Failed to authenticate user");

    println!("Logged in as user {}", user_id);

    // Save and Load Settings
    /*
    db.save_user_settings(user_id, 1, 100, 5)
        .expect("Failed to save user settings");
*/
    let settings = db.load_user_settings(user_id)
        .expect("Failed to load user settings");

    println!("User settings: {:?}", settings);

    // Update and Get Stats
    /*
    db.update_game_stats(user_id, true)
        .expect("Failed to update stats");
    */

    let _stats = db.get_user_stats(user_id)
        .expect("Failed to get user stats");

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow, // ✅ Use Glow renderer for compatibility
        vsync: true, // ✅ Ensure smooth rendering
        multisampling: 4, // ✅ Enable anti-aliasing
        ..Default::default()
    };

    log::info!("Calling eframe::run_native...");

    let result = eframe::run_native(
        "Guessing Game",
        options,
        Box::new(|_cc| {
            log::info!("Creating GuessingGameApp...");
            let app = GuessingGameApp::default();
            log::info!("GuessingGameApp successfully created!");
            Ok(Box::new(app))
        }),
    );

    if let Err(e) = result {
        log::error!("eframe failed to start: {:?}", e);
    }

    Ok(())    

}
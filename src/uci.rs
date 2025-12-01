pub enum IORunMode {
    UCI,
    UserGame,
}

pub static GAME_RUN_MODE: IORunMode = IORunMode::UCI;

pub fn parse_uci_command(input: &str) -> Result<(), String>{
    match input {
        "uci" => todo!(),
        "game" => todo!(),
        _ => Err("Unknown command".to_string())
    }
} 
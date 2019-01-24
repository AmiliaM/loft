use crate::user::UserVar;

pub enum Action {
    SendMessage(String),
    Quit,
    ChangeVariable(UserVar, Option<String>, Option<String>),
    None,
}

pub fn parse_message(message: String) -> Action {
    let mut chars = message.chars();
    return match chars.next() {
        Some(c) => match c {
            '!' => {
                match chars.collect::<String>().as_str() {
                    "joke" => {
                        Action::SendMessage("Why did the tomato blush?\nBecause it saw the salad dressing!".to_string())
                    },
                    "quit" => Action::Quit,
                    _ => {Action::None},
                }
            },
            '$' => {
                let text = chars.collect::<String>();
                let args: Vec<_> = text.split(' ').collect();
                let var = match args[0] {
                    "favfood" => UserVar::FavFood,
                    "nick" => UserVar::Nick,
                    "notif" => UserVar::Notif,
                    _ => UserVar::None,
                };
                let cmd = if args.len() > 1 {
                    Some(args[1].to_string())
                }
                else {
                    None
                };
                let value = if args.len() > 2 {
                    Some(args[2].to_string())
                }
                else {
                    None
                };
                Action::ChangeVariable(var, cmd, value)
            },
            _ => {Action::None},
        }
        _ => {Action::None},
    }
}
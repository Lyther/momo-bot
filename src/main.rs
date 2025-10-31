use rand::Rng;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("🐱 Momo the Cat Bot is waking up... meow!");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start chatting with Momo the cat!")]
    Start,
    #[command(description = "ask Momo about herself")]
    About,
    #[command(description = "make Momo purr")]
    Purr,
    #[command(description = "play with Momo")]
    Play,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Start => {
            let response = cat_speak(
                "Meow meow! *stretches and yawns* I'm Momo, the cutest cat bot in all of Telegram! \
                 *purrs* Type /help to see what we can do together, nya~! 🐾",
            );
            bot.send_message(msg.chat.id, response).await?
        }
        Command::About => {
            let response = cat_speak(
                "Meow! I'm Momo, and I love naps, treats, and headpats! *purrs* \
                 I'm a chatbot who speaks cat language... because I AM a cat! \
                 My hooman made me with Rust and teloxide. Nya~! 🐱✨",
            );
            bot.send_message(msg.chat.id, response).await?
        }
        Command::Purr => {
            let purrs = vec![
                "purrrrr purrrrr purrrrrr~ *rubs against your leg* 💕",
                "*closes eyes and purrs loudly* purrrrrrrrrr~ this feels so nice! 😸",
                "PURRRRRR PURRRRRR~ *vibrates with happiness* meow meow! 🐱",
                "*curls up in your lap* purrrrrrrrrr~ don't stop petting meow! 😻",
            ];
            let response = pick_random(&purrs);
            bot.send_message(msg.chat.id, response).await?
        }
        Command::Play => {
            let plays = vec![
                "*pounces on toy* MEOW! *bats it around* This is so fun! *zooms around* 🏃‍♀️",
                "*chases tail* meow meow meow! *stops suddenly* ...what was I doing? *yawns* 😹",
                "*eyes get big* *wiggles butt* *POUNCES* Gotcha! meow! 🎯",
                "*brings you a toy mouse* meow meow! Play with me, nya~! *bats at your hand* 🐭",
                "*hides behind curtain* ...meow? *jumps out* SURPRISE ATTACK! nya! 😼",
            ];
            let response = pick_random(&plays);
            bot.send_message(msg.chat.id, response).await?
        }
    };

    Ok(())
}

/// Adds random cat-style embellishments to text
fn cat_speak(base_text: &str) -> String {
    let mut rng = rand::thread_rng();
    let mut result = base_text.to_string();

    // Sometimes add extra meows
    if rng.gen_bool(0.3) {
        let meows = vec!["Meow!", "Nya~!", "Mew mew!", "Meow meow!"];
        result = format!("{} {}", result, pick_random(&meows));
    }

    // Sometimes add emojis
    if rng.gen_bool(0.4) {
        let emojis = vec!["🐱", "😸", "😺", "😻", "🐾", "💕"];
        result = format!("{} {}", result, pick_random(&emojis));
    }

    result
}

/// Pick a random element from a slice
fn pick_random<T>(items: &[T]) -> T
where
    T: Clone,
{
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..items.len());
    items[idx].clone()
}

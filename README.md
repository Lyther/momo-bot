# 🐱 Momo Cat Bot

A delightful Telegram chatbot that communicates in cat language style! Built with Rust using the [teloxide](https://github.com/teloxide/teloxide) framework.

## Features

- **Cat Personality**: Momo responds with meows, purrs, and playful cat behaviors
- **Interactive Commands**: Chat, play, and make Momo purr!
- **Randomized Responses**: Each interaction feels unique and spontaneous
- **Async Performance**: Built on tokio for efficient handling of multiple chats

## Commands

- `/start` - Start chatting with Momo
- `/help` - See all available commands
- `/about` - Learn more about Momo
- `/purr` - Make Momo purr (so relaxing!)
- `/play` - Play with Momo (random playful behaviors)

## Setup

### Prerequisites

- Rust (1.70 or later)
- A Telegram Bot Token from [@BotFather](https://t.me/BotFather)

### Installation

1. Clone this repository:

```bash
git clone <your-repo-url>
cd momo-bot
```

2. Copy the example environment file:

```bash
cp .env.example .env
```

3. Edit `.env` and add your Telegram Bot Token:

```yaml
TELOXIDE_TOKEN=your_actual_token_here
RUST_LOG=info
```

4. Build and run:

```bash
cargo build --release
cargo run
```

## Getting a Telegram Bot Token

1. Open Telegram and search for [@BotFather](https://t.me/BotFather)
2. Send `/newbot` command
3. Follow the instructions to name your bot
4. Copy the token provided by BotFather
5. Paste it in your `.env` file

## Project Structure

```tree
momo-bot/
├── Cargo.toml          # Rust dependencies and project metadata
├── src/
│   └── main.rs         # Main bot logic with cat personality
├── .env.example        # Example environment variables
├── .gitignore          # Git ignore rules
└── README.md           # This file
```

## Tech Stack

- **[teloxide](https://github.com/teloxide/teloxide)**: Elegant Telegram bot framework for Rust
- **[tokio](https://tokio.rs/)**: Async runtime
- **[rand](https://github.com/rust-random/rand)**: Random number generation for varied responses

## Customization

Want to customize Momo's personality? Edit the response arrays in `src/main.rs`:

```rust
let purrs = vec![
    "purrrrr purrrrr purrrrrr~ *rubs against your leg* 💕",
    // Add your own purr responses here!
];
```

You can also adjust the `cat_speak()` function to change how often Momo adds extra meows and emojis.

## Contributing

Contributions are welcome! Feel free to:

- Add new commands
- Improve cat responses
- Add more personality traits
- Fix bugs

## License

MIT License - feel free to use this code for your own cat bots!

## Acknowledgments

- Built with ❤️ and 🐱
- Powered by the amazing [teloxide](https://github.com/teloxide/teloxide) framework
- Inspired by cats everywhere who graciously allow us to serve them

---

*Meow meow! Made with purrs and whiskers~ 🐾*

# 🚀 Quick Start Guide

## Get Your Bot Running in 5 Minutes

### Step 1: Get a Telegram Bot Token

1. Open Telegram and search for `@BotFather`
2. Send `/newbot` command
3. Choose a name for your bot (e.g., "Momo Cat")
4. Choose a username (e.g., "momo_cat_bot")
5. Copy the token BotFather gives you

### Step 2: Configure the Bot

```bash
# Copy the example environment file
cp env.example .env

# Edit .env and paste your token
nano .env  # or use your favorite editor
```

Your `.env` file should look like:

```yaml
TELOXIDE_TOKEN=1234567890:ABCdefGHIjklMNOpqrsTUVwxyz
RUST_LOG=info
```

### Step 3: Run the Bot

```bash
# Build and run in one command
cargo run

# Or build a release version for better performance
cargo build --release
./target/release/momo-bot
```

You should see:

```text
🐱 Momo the Cat Bot is waking up... meow!
```

### Step 4: Chat with Your Cat

1. Open Telegram
2. Search for your bot's username
3. Send `/start`
4. Momo will greet you with a meow! 🐱

## Available Commands

- `/start` - Start chatting
- `/help` - See all commands
- `/about` - Learn about Momo
- `/purr` - Make Momo purr (random responses!)
- `/play` - Play with Momo (random playful actions!)

## Customization Ideas

### Change Momo's Personality

Edit `src/main.rs` and modify the response arrays:

```rust
let purrs = vec![
    "your custom purr here~",
    "another purr response!",
];
```

### Add New Commands

Add a new command to the enum:

```rust
#[derive(BotCommands, Clone)]
enum Command {
    // existing commands...
    #[command(description = "make Momo sleep")]
    Sleep,  // Add your new command
}
```

Then handle it in the `answer()` function!

## Troubleshooting

### "teloxide" token error

- Make sure your `.env` file exists and has the correct token
- The token should be on one line with no spaces

### Bot doesn't respond

- Check if the bot is running in your terminal
- Try sending `/start` again
- Make sure you're chatting with the correct bot

### Compile errors

- Make sure you have Rust 1.83+ installed: `rustup update`
- Try `cargo clean && cargo build`

## Next Steps

- Deploy to a server (VPS, cloud, etc.)
- Add database for user preferences
- Implement AI-powered responses
- Add more interactive features
- Connect to other services

Happy cat bot building! 🐱✨

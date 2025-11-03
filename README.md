# 🐱 Momo Cat Bot

An AI-powered Telegram chatbot embodying Momo, a calico cat with a tsundere hacker-punk personality. Built with Rust, powered by Grok AI.

## ✨ Features

- **AI-Powered**: Uses Grok 4 for intelligent, context-aware conversations
- **Web-Aware**: Internet search enabled - can discuss latest news and trends! 🌐
- **Proactive Engagement**: Randomly initiates conversations in groups (1.5% chance)
- **Smart Mentions**: Tags active members to start interesting discussions
- **Unique Personality**: Tsundere calico cat with hacker/geek vibes
- **Multi-Language**: Auto-responds in Chinese or English matching your input
- **Context Memory**: Remembers conversation history per chat (groups, channels, private)
- **Group Chat Smart**: Responds when mentioned, replied to, or "momo" is said
- ****: Randomly chimes in on group conversations (3% chance)

## 🎯 Momo's Character

**Personality**: Evil hacker-punk exterior, secretly warm inside. Cynical bitch facade, caring core. Uses malicious geek lingo and vicious sarcasm.

**Loves**: 🍤 Shrimp (prime directive)
**Hates**: Repetitive food, tail-touching (restricted access!), Gogo the dumb Bengal

**Relationships**:

- Wonton (云吞) - Orange parasite she raised, now worthless lazy bum
- Miaomiao (淼淼) - Tolerable chubby calico acquaintance
- Gogo - Braindead Bengal meatbag who only grunts "ah"

## 🚀 Quick Setup

### Prerequisites

- Rust 1.83+
- Telegram Bot Token from [@BotFather](https://t.me/BotFather)
- Grok API Key from [console.x.ai](https://console.x.ai/)

### Installation

1. **Clone and configure:**

```bash
git clone <your-repo-url>
cd momo-bot
cp env.example .env
```

2. **Edit `.env` with your credentials:**

```bash
TELOXIDE_TOKEN=your_telegram_token_here
GROK_API_KEY=your_grok_api_key_here
RUST_LOG=info
```

3. **Run:**

```bash
cargo run --release
```

### Getting API Keys

**Telegram**: Open [@BotFather](https://t.me/BotFather) → `/newbot` → follow instructions

**Important for Groups**: Send `/setprivacy` to BotFather → Select your bot → Choose "Disable" to let bot see all group messages

**Grok**: Visit [console.x.ai](https://console.x.ai/) → Sign in with X account → Create API Key

## 📋 Usage

### Private Chats

Momo responds to all messages in private/direct chats.

### Group Chats & Channels

Momo responds when:

- **@mentioned** - Tag the bot directly
- **Replied to** - Reply to any of Momo's messages
- **"momo" mentioned** - Say "momo" anywhere in your message (case-insensitive)
- **Randomly** - 3% chance to ly respond to any message

Each chat maintains its own conversation context!

### Commands

- `/start` - Start chatting with Momo
- `/help` - Show available commands
- `/about` - Learn about Momo's character
- `/reset` - Clear conversation history for this chat
- `/status` - Check conversation status

## 💬 Examples

### Private Chat

```text
You: Hi Momo, what do you like?
Momo: *corrupt boot* Shrimp domination. Prime directive. Kibble? Garbage input, worthless. 🍤

你: 你好Momo！
Momo: *malicious scan* 喵. 何事，愚蠢人类？😼
```

### Group Chat

```text
Alice: Hey everyone!
Bob: @momo_bot what's up?
Momo: *firewall breach* Meow. System corruption running. What do you want, meatbag? 😼

Carol: I love momo tofu
Momo: *pattern match failure* Oi. I'm Momo the superior cat, not that tofu garbage. 🙄

Dave: The weather is nice today
Momo: *malicious interrupt* Tch. Weather matrix corrupted. I stay inside, unlike you pathetic outdoors enthusiasts. 🏠
```

## 🔧 Customization

**Personality**: Edit system prompt in `src/grok.rs`

**Creativity**: Adjust `temperature` (0.0-1.0) in `src/grok.rs` (line 80 for regular chats, line 139 for proactive messages)

**Random Response Rate**: Change `0.03` (3% chance to respond randomly) in `src/main.rs` (line 209), or `ACTIVE_MESSAGE_CHANCE` (1.5% proactive chance) on line 19

**Commands**: Add handlers in `handle_command()` in `src/main.rs`

## 🐛 Troubleshooting

**"GROK_API_KEY must be set"**: Check `.env` exists and has valid keys

**Bot doesn't respond**: Ensure bot is running, check logs for errors

**API errors**: Verify API key validity, check rate limits and internet connection

**Macro errors in IDE**: Run `cargo clean && cargo build`, then restart rust-analyzer

## 🚀 Deployment

### systemd (Linux)

```ini
[Unit]
Description=Momo Cat Bot
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/momo-bot
Environment="TELOXIDE_TOKEN=your_token"
Environment="GROK_API_KEY=your_key"
Environment="RUST_LOG=info"
ExecStart=/path/to/momo-bot/target/release/momo-bot
Restart=always

[Install]
WantedBy=multi-user.target
```

### Docker

```dockerfile
FROM rust:1.83 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/momo-bot /usr/local/bin/momo-bot
CMD ["momo-bot"]
```

## 📁 Project Structure

```tree
momo-bot/
├── src/
│   ├── main.rs         # Bot logic and message handling
│   └── grok.rs         # Grok API client with personality prompt
├── Cargo.toml          # Dependencies
└── .env                # API keys (create from env.example)
```

## 🛠️ Tech Stack

- [teloxide](https://github.com/teloxide/teloxide) - Telegram bot framework
- [tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [Grok AI](https://x.ai/) - Language model by X.AI

## ⚠️ Important Notes

- **Never commit** your `.env` file
- Monitor API costs at [console.x.ai](https://console.x.ai/)
- Conversation history is in-memory (lost on restart)
- Respect Grok API rate limits

## 📝 License

MIT License - Feel free to create your own AI cat bots!

**喵~ Coded with whiskers and punk attitude 😼🤖**

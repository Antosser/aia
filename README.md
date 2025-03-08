# AIA Terminal Assistant 🚀

AIA (AI Assistant) is a terminal-based tool that leverages OpenAI's GPT models to assist you with command-line tasks, answer questions, and provide context-aware assistance based on your current working directory and piped input. 🤖💻

---

## Features ✨

- **Context Awareness**: AIA automatically detects the files in your current directory and uses this information to provide relevant assistance. 📂🔍
- **Interactive CLI**: Engage in a conversational interface to execute commands, ask follow-up questions, or quit the session. 💬🔄
- **Piped Input Support**: Pass input directly to AIA via pipes (e.g., `cat file.txt | aia`). 📥🔗
- **Customizable Configuration**: Set your OpenAI API key and preferred model in the configuration file. ⚙️🔑

---

## Installation 📥

To install AIA, follow these steps:

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/aia.git
   cd aia
   ```

2. Build and install the project using Cargo:
   ```bash
   cargo install --path .
   ```

This will compile the project and install the `aia` binary on your system. 🛠️✅

---

## Configuration ⚙️

Before using AIA, you need to set up your OpenAI API key:

1. Create a configuration file at `~/.config/aia/config.toml`.
2. Add your OpenAI API key and preferred model to the file:
   ```toml
   openai_token = "your_openai_api_key_here"
   openai_model = "gpt-4"  # or any other supported model
   ```

---

## Usage 🚀

Run AIA in your terminal:

```bash
aia
```

You can also pass input directly via pipes:

```bash
cat file.txt | aia
```

---

### Interactive Commands 🕹️

- **Input**: Type your query or command request. ⌨️
- **Execute Command**: AIA will suggest commands, and you can choose to execute them, ask follow-up questions, or quit. 🛠️
- **Follow-up**: Continue the conversation or refine your request. 🔄
- **Quit**: Exit the AIA session. 🛑

---

## Example 🌟

```bash
$ aia
AIA Terminal Assistant 🚀

Input: How can I list all files in this directory?
Command: ls
Pick an action:
  execute: Execute 🛠️
  follow: Follow-up 🔄
  quit: Quit 🛑
```

---

## Dependencies 📦

- **async-openai**: For interacting with OpenAI's API. 🤖
- **cliclack**: For building the interactive CLI interface. 🖥️
- **tokio**: For asynchronous runtime support. ⚡
- **serde_json**: For parsing JSON responses. 📄

---

## Contributing 🤝

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes. 🐛🔧

---

## License 📜

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

Enjoy using AIA to streamline your terminal workflow! 🎯✨

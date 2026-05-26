## How to Use With Any Model

One of the main goals of this framework is flexibility. You do **not** need a specially fine-tuned model to use tools. If your model is capable of following instructions and generating the correct command formats, it can use this system.

### Requirements

- A local model server running (llama.cpp, Ollama, vLLM, etc.)
- A model that can reliably follow instructions
- The ability to edit the system prompt

### Quick Setup

1. **Point the framework at your model**  
   Edit `config.toml` and update the `[endpoint]` section with your model’s address and name:

   ```toml
   [endpoint]
   url = "http://localhost:8080/v1/chat/completions"
   model = "your-model-name"
   temperature = 0.7
   max_tokens = 2048
   ```
2. Use (or customize) the system prompt
   The framework includes a basic system prompt that teaches the model the available tool formats. You can find it in the prompts/ folder.You can replace or modify this prompt to better fit your model. The most important thing is clearly explaining the tool formats:
   COMMAND: for simple one-shot commands
   SESSION:NAME for persistent/interactive sessions
   JSON_TOOL: for structured tool calls (if using JSON mode)
  
3. Start the framework
   ```Bash
   cargo run
   ```
   Talk to your model normally
   Just type what you want it to do. If the model understands the tool formats in the system prompt, it will use them.
  
### Tips for Best Results
  
Raw text methods (COMMAND: and SESSION:) generally work better with most models than JSON.
Stronger models (7B+) tend to follow tool instructions more reliably.
If the model struggles, try making the tool format instructions in the system prompt more explicit or repetitive.
You can use completely different models for the main agent and the summarizer by editing the [endpoint] and [summarizer] sections.

Limitations
While most reasonably capable models can use tools through this framework, very small or heavily quantized models may struggle with consistent tool use and multi-step reasoning.

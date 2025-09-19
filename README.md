# Basic Number Guessing Game (Rust)

A simple terminal-based number guessing game written in Rust. The game picks a random number between 1 and 100, and you try to guess it. Feedback appears in color: green on success and red for guidance/errors.


## Prerequisites
- Rust toolchain (rustc and cargo). Install from https://rustup.rs
- A terminal that supports ANSI colors.
  - On Windows 10+ the default terminal typically works. If colors don’t show, try running in Windows Terminal or enable ANSI support.


## How to build and run
1. Clone or open this project.
2. Build:
   - cargo build
3. Run:
   - cargo run

You should see:
- "Welcome to the number guessing game!"
- Prompts to enter a guess between 1 and 100.
- Messages indicating if your guess is too low, too high, or correct.


## How to play
- Enter an integer between 1 and 100 and press Enter.
- If the guess is not a number or outside the range, the program will show a red message and ask again.
- When you guess the secret number, you’ll see a green congratulatory message and the program exits.


## Troubleshooting
- Colors not appearing: Ensure your terminal supports ANSI colors. On Windows, use Windows Terminal or PowerShell 7+.
- Build issues: Run cargo clean and then cargo build again. Make sure you’re on a recent stable Rust.


## Where things are
- Source code: src/main.rs
- Dependencies: Cargo.toml (rand for randomness, colored for colored output)


## Essential Rust concepts used in this project (beginner-friendly)

This tiny project demonstrates several core Rust concepts. Below are short explanations tied directly to the code in src/main.rs.

1) Shadowing
- In the game, we read the user input into a mutable String named entrada, then we trim it (remove whitespace) and reuse the same name for the trimmed view:
  - let mut entrada = String::new();
  - ... read_line(&mut entrada) ...
  - let entrada = entrada.trim();
- The second let entrada shadows the first. After this line, entrada has a new type (&str) and value (trimmed). Shadowing allows you to transform a value and keep using the same name in a controlled, scope-limited way.

2) Match expressions
- Parsing the user input from text to a number returns a Result. We handle both success and failure using match:
  - let adivinanza: u32 = match entrada.parse() {
      Ok(num) => num,
      Err(_) => { println!("Invalid input. Please enter a number.".red()); continue; }
    };
- We also compare the guess to the secret number using cmp, which returns an Ordering (Less, Greater, Equal). We match on it to print the appropriate message:
  - match adivinanza.cmp(&numero_secreto) { Ordering::Less => ..., Ordering::Greater => ..., Ordering::Equal => ... }
- match lets you branch on different variants or patterns in a clear, exhaustive way.

3) Result handling
- Parsing returns Result<u32, E>. We use match to convert Ok(num) into the number and handle Err by printing a message and continuing the loop:
  - Ok(num) => num
  - Err(_) => continue
- This shows basic error handling without panicking: gracefully recover and ask the user again.

4) Borrowing
- Instead of moving values, Rust encourages borrowing with references (&T for shared, &mut T for mutable):
  - io::stdin().read_line(&mut entrada) borrows entrada mutably so read_line can fill the String without taking ownership.
  - adivinanza.cmp(&numero_secreto) borrows numero_secreto (and cmp also borrows adivinanza implicitly as &self) to compare without moving values.
  - (Range check) (1..=100).contains(&adivinanza) borrows adivinanza to test membership.
- Borrowing enables safe, efficient access without copying or transferring ownership.

5) if-let pattern
- When reading from stdin, we only care about the Err case to report an error and continue. if let provides concise pattern matching for a single pattern:
  - if let Err(e) = io::stdin().read_line(&mut entrada) {
      eprintln!("{}", format!("Error reading input: {}", e).red());
      continue;
    }
- This is shorthand for a match that handles Err and ignores Ok.

6) Loop control (loop, break, continue)
- We use an infinite loop to keep prompting until the user guesses correctly:
  - loop { ... }
- continue restarts the loop early when input is invalid or out of range.
- break exits the loop when the guess is correct, which ends the program.


## Why these crates?
- rand: Provides thread_rng and gen_range(1..=100) to pick the secret number.
- colored: Provides .red() and .green() methods to colorize terminal output for better UX.


## Next steps to explore
- Limit the number of attempts and show a score.
- Add difficulty levels (different ranges).
- Write tests for input parsing logic (extract functions) and comparison behavior.

use rand::Rng;
use std::cmp::Ordering;
use std::io;
use colored::*;

fn main() {
    println!("Welcome to the number guessing game!");
    println!("I've chosen a number between 1 and 100. Can you guess it?");

    let numero_secreto = rand::thread_rng().gen_range(1..=100);

    loop {
        println!("Please enter your guess (1-100):");

        let mut entrada = String::new();
        if let Err(e) = io::stdin().read_line(&mut entrada) {
            eprintln!("{}", format!("Error reading input: {}", e).red());
            continue;
        }

        let entrada = entrada.trim();
        let adivinanza: u32 = match entrada.parse() {
            Ok(num) => num,
            Err(_) => {
                println!("{}", "Invalid input. Please enter a number.".red());
                continue;
            }
        };

        if !(1..=100).contains(&adivinanza) {
            println!("{}", "Please enter a number between 1 and 100.".red());
            continue;
        }

        match adivinanza.cmp(&numero_secreto) {
            Ordering::Less => println!("{}", "Too low. Try again!".red()),
            Ordering::Greater => println!("{}", "Too high. Try again!".red()),
            Ordering::Equal => {
                println!("{}", format!("Congratulations! You guessed the number: {}", numero_secreto).green());
                break;
            }
        }
    }
}

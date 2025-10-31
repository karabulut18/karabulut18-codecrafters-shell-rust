use std::io::{self, Write};


fn main()
{
    loop
    {
        print!("$ ");
        io::stdout().flush().unwrap();    
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let mut parts = input.trim().split_whitespace();
        let command = parts.next();
        match command
        {
            Some("exit") =>
            {
                if let Some(arg) = parts.next()
                {
                    if let Ok(exit_code) = arg.parse::<i32>()
                    {
                        std::process::exit(exit_code);
                    }
                    else
                    {
                        std::process::exit(1);
                    }
                }
                else
                {
                    std::process::exit(0);
                }
            }
            Some("echo") =>
            {
                let output = parts.collect::<Vec<&str>>().join(" ");
                println!("{}", output);
            }
            Some("type") =>
            {
                if let Some(arg) = parts.next()
                {
                    match arg
                    {
                        "echo" | "exit" | "type" => println!("{} is a shell builtin", arg),
                        _ => println!("{}: not found", arg),
                    }
                }
            }
            Some(cmd) =>
            {
                println!("{}: command not found", cmd);
            }
            None =>{}
        }
    }
}
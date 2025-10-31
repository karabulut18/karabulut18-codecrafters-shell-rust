use std::io::{self, Write};
use std::env;
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;


fn find_executable_in_path(name: &str) -> Option<PathBuf>
{
    // Improvement: Use `ok().and_then()` for more idiomatic Option/Result handling
    env::var("PATH").ok().and_then(|path_var| {
        for path in env::split_paths(&path_var)
        {
            let full_path = path.join(name);
            
            // 1. Check if the path points to a file
            if full_path.is_file()
            {
                // 2. Check if we have execute permission
                if let Ok(metadata) = full_path.metadata() {
                    // This checks if the executable bit is set for the current user.
                    // The standard mode constant for execute permission for the owner is 0o100.
                    // This is a simple, common way to check for execution permission.
                    if metadata.permissions().mode() & 0o111 != 0 {
                        return Some(full_path);
                    }
                }
            }
        }
        None
    })
}

// execute function
// catch the output and print
fn execute(command: &str, args: &[&str])
{
    if let Some(path) = find_executable_in_path(command){
        let mut child = std::process::Command::new(path).args(args).spawn().expect("Failed to execute command");
        let _  = child.wait();
    }
    else
    {
        println!("{}: command not found", command);
    }
}

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
                        _ =>
                        {
                          if let Some(path) = find_executable_in_path(arg)
                          {
                            println!("{} is {}", arg, path.display());
                          }
                          else
                          {
                            println!("{}: not found", arg);
                          }
                        }
                    }
                }
            }
            _ =>
            {
                execute(command.unwrap(), parts.collect::<Vec<&str>>().as_slice());
            }
        }
    }
}
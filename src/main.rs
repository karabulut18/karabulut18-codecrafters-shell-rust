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
    if find_executable_in_path(command).is_some() {
        
        // FIX: Use `Command::new(cmd)` (the filename) instead of `Command::new(path)`.
        // This relies on the system's internal PATH search (which we verified worked)
        // and correctly sets argv[0] to the filename as the tester expects.
        match std::process::Command::new(command).args(args).spawn() {
            Ok(mut child) => {
                // Wait for the command to finish and capture its exit status
                match child.wait() {
                    Ok(_) => {},
                    Err(e) => eprintln!("Execution error: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to execute {}: {}", command, e),
        }
    } else {
        println!("{}: command not found", command);
    }
}

fn change_directory(path: &str)
{
    // if it is absolute path, check if the directory is exist
    if path.starts_with('/')
    {
        if let Err(_) = env::set_current_dir(path)
        {
            eprintln!("{}: no such file or directory", path);
        }
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
            Some("pwd") =>
            {
                if let Ok(current_dir) = env::current_dir()
                {
                    println!("{}", current_dir.display());
                }
                else
                {
                    eprintln!("Failed to get current directory");
                }
            }
            Some("cd") =>
            {
                if let Some(arg) = parts.next()
                {
                    change_directory(arg);
                }
            }
            Some("type") =>
            {
                if let Some(arg) = parts.next()
                {
                    match arg
                    {
                        "echo" | "exit" | "type" | "pwd" | "cd" => println!("{} is a shell builtin", arg),
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
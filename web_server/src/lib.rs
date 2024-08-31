use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

// ThreadPool struct manages a pool of threads.
pub struct ThreadPool {
    workers: Vec<Worker>,              // Vector of workers (threads)
    sender: Option<mpsc::Sender<Job>>, // Sender for sending jobs to the worker threads
}

// Type alias for a job to be executed by the thread pool.
// The job is a boxed closure that takes no parameters and returns nothing.
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0); // Ensure that the pool size is greater than 0

        // Create a channel for sending jobs to workers.
        let (sender, receiver) = mpsc::channel();

        // Wrap the receiver in an Arc and a Mutex to safely share it across threads.
        let receiver = Arc::new(Mutex::new(receiver));

        // Pre-allocate space for the workers.
        let mut workers = Vec::with_capacity(size);

        // Create worker threads and add them to the pool.
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        // Return the ThreadPool instance with the workers and the sender.
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Execute a function using the thread pool.
    ///
    /// The function must implement the `FnOnce` trait, which means it can be called once,
    /// and it must be `Send` to move between threads safely and `'static` to ensure
    /// it lives long enough to be executed.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Box the function to turn it into a `Job`.
        let job = Box::new(f);

        // Send the job to the worker threads via the channel.
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    /// The `Drop` trait implementation ensures that when the ThreadPool goes out of scope,
    /// all threads are properly shut down.
    fn drop(&mut self) {
        // Close the sending side of the channel to signal the workers to shut down.
        drop(self.sender.take());

        // Join each worker thread to ensure they have finished before the pool is destroyed.
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            // If the worker thread exists, join it to wait for its completion.
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// Worker struct represents a single thread in the pool.
struct Worker {
    id: usize,                              // Unique ID of the worker
    thread: Option<thread::JoinHandle<()>>, // The thread handle
}

impl Worker {
    /// Create a new worker and spawn a thread to listen for jobs.
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        // Spawn a new thread and move the receiver into the thread's closure.
        let thread = thread::spawn(move || loop {
            // Lock the receiver to get a job from the channel.
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    // Execute the job.
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    // If the channel is closed, exit the loop and end the thread.
                    break;
                }
            }
        });

        // Return the Worker instance with its thread.
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
    _workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    _id: usize,
    _thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let _thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();

            job();
        });

        Worker { _id, _thread }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut _workers = Vec::with_capacity(size);

        for id in 0..size {
            _workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { _workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

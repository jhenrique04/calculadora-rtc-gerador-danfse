use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// estado de readiness thread-safe. Deve virar `false` antes do graceful
/// shutdown para que /health/ready passe a responder 503
#[derive(Debug, Clone)]
pub struct Readiness(Arc<AtomicBool>);

impl Readiness {
    pub fn new(ready: bool) -> Self {
        Self(Arc::new(AtomicBool::new(ready)))
    }

    pub fn set_ready(&self, ready: bool) {
        self.0.store(ready, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

impl Default for Readiness {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estado_inicial_e_flips() {
        let r = Readiness::new(true);
        assert!(r.is_ready());
        r.set_ready(false);
        assert!(!r.is_ready());
        let copia = r.clone();
        copia.set_ready(true);
        assert!(r.is_ready());
        assert!(!Readiness::new(false).is_ready());
    }

    #[test]
    fn thread_safe() {
        let r = Readiness::new(true);
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let r = r.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        r.set_ready(i % 2 == 0);
                        let _ = r.is_ready();
                    }
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().is_ok());
        }
    }
}

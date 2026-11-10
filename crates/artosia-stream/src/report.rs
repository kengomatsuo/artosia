//! Printing the JSON lines. Everything goes through the redactor first, so a stream key can
//! never reach stdout, whichever code path wrote the line.

use crate::redact::Redactor;
use crate::status;
use serde_json::Value;
use std::io::Write;
use std::sync::Mutex;

pub struct Reporter {
    redactor: Redactor,
    lock: Mutex<()>,
}

impl Reporter {
    pub fn new(redactor: Redactor) -> Self {
        Self {
            redactor,
            lock: Mutex::new(()),
        }
    }

    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// Prints one line. A closed stdout (the operator is gone) is ignored: the stream goes on.
    pub fn emit(&self, value: &Value) {
        let line = self.redactor.clean(&value.to_string());
        let _guard = self.lock.lock();
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }

    pub fn problem(&self, message: &str, detail: Option<&str>) {
        self.emit(&status::problem(message, detail));
    }

    pub fn info(&self, message: &str) {
        self.emit(&status::info(message));
    }
}

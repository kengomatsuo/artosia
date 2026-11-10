//! Keeping stream keys out of everything the program prints.

use crate::args::Destination;

const MASK: &str = "***";

/// The URL with its secret parts replaced by `***`.
///
/// RTMP: the last path segment (the stream key), any query value, and a password in the
/// user-info part. SRT: every query value (`passphrase`, `streamid`).
pub fn redact_url(url: &str) -> String {
    let (head, query) = match url.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (url, None),
    };
    let mut out = redact_head(head);
    if let Some(q) = query {
        out.push('?');
        let parts: Vec<String> = q
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((k, _)) => format!("{k}={MASK}"),
                None => MASK.to_string(),
            })
            .collect();
        out.push_str(&parts.join("&"));
    }
    out
}

fn redact_head(head: &str) -> String {
    let Some((scheme, rest)) = head.split_once("://") else {
        return MASK.to_string();
    };
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (rest, None),
    };
    let authority = match authority.rsplit_once('@') {
        Some((user, host)) => {
            let user = user.split_once(':').map_or(user, |(name, _)| name);
            format!("{user}:{MASK}@{host}")
        }
        None => authority.to_string(),
    };
    let mut out = format!("{scheme}://{authority}");
    if let Some(path) = path {
        let mut segments: Vec<&str> = path.split('/').collect();
        // The stream key is the last segment of an RTMP path ("live2/KEY").
        if scheme.starts_with("rtmp")
            && let Some(last) = segments.last_mut()
            && !last.is_empty()
        {
            *last = MASK;
        }
        out.push('/');
        out.push_str(&segments.join("/"));
    }
    out
}

/// Knows every secret of this run and scrubs any text before it is printed.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    urls: Vec<String>,
    secrets: Vec<String>,
}

impl Redactor {
    pub fn new(destinations: &[Destination]) -> Self {
        let mut r = Self::default();
        for d in destinations {
            match d {
                Destination::Rtmp(u) | Destination::Srt(u) => r.add_url(u),
                Destination::Record(_) => {}
            }
        }
        r
    }

    fn add_url(&mut self, url: &str) {
        self.urls.push(url.to_string());
        let (head, query) = match url.split_once('?') {
            Some((h, q)) => (h, Some(q)),
            None => (url, None),
        };
        if let Some(q) = query {
            for pair in q.split('&') {
                if let Some((_, v)) = pair.split_once('=') {
                    self.add_secret(v);
                }
            }
        }
        if let Some((_, rest)) = head.split_once("://")
            && let Some((authority, path)) = rest.split_once('/')
        {
            if let Some((user, _)) = authority.rsplit_once('@')
                && let Some((_, pass)) = user.split_once(':')
            {
                self.add_secret(pass);
            }
            if head.starts_with("rtmp")
                && let Some(key) = path.rsplit('/').next()
            {
                self.add_secret(key);
            }
        }
    }

    fn add_secret(&mut self, secret: &str) {
        if !secret.is_empty() && !self.secrets.iter().any(|s| s == secret) {
            self.secrets.push(secret.to_string());
        }
    }

    /// The text with every known URL shortened to its redacted form and every secret masked.
    pub fn clean(&self, text: &str) -> String {
        let mut out = text.to_string();
        for url in &self.urls {
            if out.contains(url.as_str()) {
                out = out.replace(url.as_str(), &redact_url(url));
            }
        }
        // Longest first, so a secret that contains another one is masked whole.
        let mut secrets: Vec<&String> = self.secrets.iter().collect();
        secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        for s in secrets {
            if out.contains(s.as_str()) {
                out = out.replace(s.as_str(), MASK);
            }
        }
        out
    }

    /// The name shown for a destination: its redacted URL, or the file.
    pub fn label(&self, d: &Destination) -> String {
        match d {
            Destination::Rtmp(u) | Destination::Srt(u) => redact_url(u),
            Destination::Record(p) => format!("file {}", p.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtmp_key_is_hidden() {
        assert_eq!(
            redact_url("rtmp://a.rtmp.youtube.com/live2/abcd-efgh-1234"),
            "rtmp://a.rtmp.youtube.com/live2/***"
        );
        assert_eq!(
            redact_url("rtmps://live-api-s.facebook.com:443/rtmp/FB-123-0-Abc"),
            "rtmps://live-api-s.facebook.com:443/rtmp/***"
        );
    }

    #[test]
    fn query_values_and_passwords_are_hidden() {
        assert_eq!(
            redact_url("rtmp://h/app/key?token=SECRET&x=1"),
            "rtmp://h/app/***?token=***&x=***"
        );
        assert_eq!(
            redact_url("rtmp://user:pw@h/app/key"),
            "rtmp://user:***@h/app/***"
        );
        assert_eq!(
            redact_url("srt://h:9000?passphrase=hunter2hunter2&streamid=room"),
            "srt://h:9000?passphrase=***&streamid=***"
        );
    }

    #[test]
    fn nonsense_is_masked_whole() {
        assert_eq!(redact_url("not a url"), "***");
    }

    #[test]
    fn the_key_never_survives_in_free_text() {
        let url = "rtmp://a.rtmp.youtube.com/live2/abcd-efgh-1234";
        let r = Redactor::new(&[Destination::Rtmp(url.to_string())]);
        let msg = format!("Could not connect to {url}: refused (key abcd-efgh-1234)");
        let clean = r.clean(&msg);
        assert!(!clean.contains("abcd-efgh-1234"), "{clean}");
        assert!(clean.contains("rtmp://a.rtmp.youtube.com/live2/***"));
    }

    #[test]
    fn the_key_alone_is_masked_when_the_url_is_split() {
        let r = Redactor::new(&[Destination::Rtmp("rtmp://h/live2/KEY12345".to_string())]);
        assert_eq!(r.clean("stream KEY12345 failed"), "stream *** failed");
    }

    #[test]
    fn srt_passphrase_is_masked_in_text() {
        let r = Redactor::new(&[Destination::Srt(
            "srt://h:1?passphrase=longsecretvalue".to_string(),
        )]);
        assert!(
            !r.clean("bad passphrase longsecretvalue")
                .contains("longsecret")
        );
    }

    #[test]
    fn labels_never_carry_the_key() {
        let r = Redactor::default();
        let l = r.label(&Destination::Rtmp("rtmp://h/app/KEY".into()));
        assert!(!l.contains("KEY"));
        assert!(
            r.label(&Destination::Record("a.mp4".into()))
                .contains("a.mp4")
        );
    }
}

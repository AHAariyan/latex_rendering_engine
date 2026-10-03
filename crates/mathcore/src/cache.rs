//! A layout cache for hosts that draw the same formulas again and again: a
//! list that scrolls, a widget that recomposes, a document that reflows.
//!
//! Keys are the whole request (source, size, mode, colour, width, macros,
//! hit testing, budget), so a hit is always exactly what `render` would
//! return. Least recently used entries go first. The cache is per font: keep
//! one beside each `MathFont`.

use crate::{render, DisplayList, MathFont, RenderOptions, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct LayoutCache {
    inner: Mutex<Inner>,
}

struct Inner {
    capacity: usize,
    tick: u64,
    map: HashMap<String, (Arc<DisplayList>, u64)>,
    hits: u64,
    misses: u64,
}

/// Counters for tuning the capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new(256)
    }
}

impl LayoutCache {
    /// A cache holding up to `capacity` layouts; 0 disables it.
    pub fn new(capacity: usize) -> Self {
        LayoutCache {
            inner: Mutex::new(Inner {
                capacity,
                tick: 0,
                map: HashMap::new(),
                hits: 0,
                misses: 0,
            }),
        }
    }

    /// `render`, answered from the cache when the same request was seen before.
    /// Errors are not cached.
    pub fn render(&self, font: &MathFont<'_>, tex: &str, opts: &RenderOptions) -> Result<Arc<DisplayList>> {
        let key = key(tex, opts);
        {
            let mut inner = self.lock();
            if inner.capacity == 0 {
                drop(inner);
                return render(font, tex, opts).map(Arc::new);
            }
            inner.tick += 1;
            let tick = inner.tick;
            if let Some((dl, last)) = inner.map.get_mut(&key) {
                *last = tick;
                let dl = Arc::clone(dl);
                inner.hits += 1;
                return Ok(dl);
            }
            inner.misses += 1;
        }
        // Lay out without holding the lock, so other threads are not blocked.
        let dl = Arc::new(render(font, tex, opts)?);
        let mut inner = self.lock();
        if inner.map.len() >= inner.capacity {
            if let Some(oldest) = inner.map.iter().min_by_key(|(_, (_, t))| *t).map(|(k, _)| k.clone()) {
                inner.map.remove(&oldest);
            }
        }
        let tick = inner.tick;
        inner.map.insert(key, (Arc::clone(&dl), tick));
        Ok(dl)
    }

    /// Changes the capacity, dropping the least recently used entries that no
    /// longer fit.
    pub fn set_capacity(&self, capacity: usize) {
        let mut inner = self.lock();
        inner.capacity = capacity;
        while inner.map.len() > capacity {
            let Some(oldest) = inner.map.iter().min_by_key(|(_, (_, t))| *t).map(|(k, _)| k.clone()) else {
                break;
            };
            inner.map.remove(&oldest);
        }
    }

    pub fn clear(&self) {
        self.lock().map.clear();
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.lock();
        CacheStats {
            entries: inner.map.len(),
            hits: inner.hits,
            misses: inner.misses,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves the map consistent (every
        // mutation is a single call), so a poisoned lock is still usable.
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn key(tex: &str, o: &RenderOptions) -> String {
    format!(
        "{tex}\u{0}{}\u{0}{}\u{0}{:?}\u{0}{:?}\u{0}{}\u{0}{:?}\u{0}{:?}",
        o.font_size.to_bits(),
        o.display_mode,
        o.color,
        o.line_break.map(|l| (l.max_width.to_bits(), l.indent.to_bits())),
        o.hit_testing,
        o.budget,
        o.macros,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_return_what_render_returns() {
        let font = crate::bundled::font().unwrap();
        let cache = LayoutCache::new(2);
        let opts = RenderOptions::default();
        let a = cache.render(&font, r"\frac{a}{b}", &opts).unwrap();
        let again = cache.render(&font, r"\frac{a}{b}", &opts).unwrap();
        assert!(Arc::ptr_eq(&a, &again));
        assert_eq!(*a, render(&font, r"\frac{a}{b}", &opts).unwrap());
        // A different size is a different entry.
        let big = cache
            .render(
                &font,
                r"\frac{a}{b}",
                &RenderOptions {
                    font_size: 64.0,
                    ..opts.clone()
                },
            )
            .unwrap();
        assert!(big.width > a.width);
        assert_eq!(
            cache.stats(),
            CacheStats {
                entries: 2,
                hits: 1,
                misses: 2
            }
        );
    }

    #[test]
    fn least_recently_used_goes_first() {
        let font = crate::bundled::font().unwrap();
        let cache = LayoutCache::new(2);
        let o = RenderOptions::default();
        cache.render(&font, "a", &o).unwrap();
        cache.render(&font, "b", &o).unwrap();
        cache.render(&font, "a", &o).unwrap(); // a is now newer than b
        cache.render(&font, "c", &o).unwrap(); // evicts b
        let before = cache.stats().hits;
        cache.render(&font, "a", &o).unwrap();
        assert_eq!(cache.stats().hits, before + 1);
        cache.render(&font, "b", &o).unwrap();
        assert_eq!(cache.stats().hits, before + 1, "b should have been evicted");
    }

    #[test]
    fn errors_are_not_cached_and_zero_disables() {
        let font = crate::bundled::font().unwrap();
        let cache = LayoutCache::new(0);
        assert!(cache.render(&font, r"\nosuch", &RenderOptions::default()).is_err());
        cache.render(&font, "x", &RenderOptions::default()).unwrap();
        assert_eq!(cache.stats().entries, 0);
    }
}

//! Placeholder text in the familiar pseudo-Latin of "Lorem ipsum".

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

pub const MAX_PARAGRAPHS: usize = 100;
pub const MAX_SENTENCES: usize = 500;
pub const MAX_WORDS: usize = 5000;

const OPENING: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet,",
    "consectetur",
    "adipiscing",
    "elit",
];

const WORDS: &[&str] = &[
    "a",
    "ac",
    "accumsan",
    "ad",
    "aenean",
    "aliquam",
    "aliquet",
    "amet",
    "ante",
    "arcu",
    "at",
    "auctor",
    "augue",
    "bibendum",
    "blandit",
    "commodo",
    "condimentum",
    "congue",
    "consequat",
    "convallis",
    "cras",
    "cursus",
    "dapibus",
    "diam",
    "dictum",
    "dignissim",
    "dis",
    "donec",
    "dui",
    "duis",
    "egestas",
    "eget",
    "eleifend",
    "elementum",
    "enim",
    "erat",
    "eros",
    "est",
    "et",
    "etiam",
    "eu",
    "euismod",
    "facilisis",
    "fames",
    "faucibus",
    "felis",
    "fermentum",
    "feugiat",
    "fringilla",
    "fusce",
    "gravida",
    "habitant",
    "hendrerit",
    "iaculis",
    "id",
    "imperdiet",
    "in",
    "integer",
    "interdum",
    "justo",
    "lacinia",
    "lacus",
    "laoreet",
    "lectus",
    "leo",
    "libero",
    "ligula",
    "lobortis",
    "luctus",
    "maecenas",
    "magna",
    "malesuada",
    "massa",
    "mattis",
    "mauris",
    "metus",
    "mi",
    "molestie",
    "mollis",
    "morbi",
    "nam",
    "nec",
    "neque",
    "netus",
    "nibh",
    "nisi",
    "nisl",
    "non",
    "nulla",
    "nullam",
    "nunc",
    "odio",
    "orci",
    "ornare",
    "pellentesque",
    "pharetra",
    "phasellus",
    "placerat",
    "porta",
    "porttitor",
    "posuere",
    "praesent",
    "pretium",
    "proin",
    "pulvinar",
    "purus",
    "quam",
    "quis",
    "quisque",
    "rhoncus",
    "risus",
    "rutrum",
    "sagittis",
    "sapien",
    "scelerisque",
    "sed",
    "sem",
    "semper",
    "senectus",
    "sodales",
    "sollicitudin",
    "suscipit",
    "suspendisse",
    "tellus",
    "tempor",
    "tempus",
    "tincidunt",
    "tortor",
    "tristique",
    "turpis",
    "ullamcorper",
    "ultrices",
    "ultricies",
    "urna",
    "ut",
    "varius",
    "vehicula",
    "vel",
    "velit",
    "venenatis",
    "vestibulum",
    "vitae",
    "vivamus",
    "viverra",
    "volutpat",
    "vulputate",
];

const SENTENCE_WORDS: std::ops::RangeInclusive<usize> = 6..=14;
const PARAGRAPH_SENTENCES: std::ops::RangeInclusive<usize> = 4..=7;
/// Roughly one word in ten is followed by a comma, which reads more like prose.
const COMMA_CHANCE: f64 = 0.1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoremUnit {
    #[default]
    Paragraphs,
    Sentences,
    Words,
}

impl LoremUnit {
    pub const ALL: [LoremUnit; 3] = [
        LoremUnit::Paragraphs,
        LoremUnit::Sentences,
        LoremUnit::Words,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            LoremUnit::Paragraphs => "Paragraphs",
            LoremUnit::Sentences => "Sentences",
            LoremUnit::Words => "Words",
        }
    }

    pub const fn max_count(self) -> usize {
        match self {
            LoremUnit::Paragraphs => MAX_PARAGRAPHS,
            LoremUnit::Sentences => MAX_SENTENCES,
            LoremUnit::Words => MAX_WORDS,
        }
    }

    pub const fn default_count(self) -> usize {
        match self {
            LoremUnit::Paragraphs => 3,
            LoremUnit::Sentences => 5,
            LoremUnit::Words => 50,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoremSpec {
    pub unit: LoremUnit,
    pub count: usize,
    /// Opens with "Lorem ipsum dolor sit amet, consectetur adipiscing elit".
    pub should_start_with_lorem: bool,
}

impl Default for LoremSpec {
    fn default() -> Self {
        Self {
            unit: LoremUnit::default(),
            count: LoremUnit::default().default_count(),
            should_start_with_lorem: true,
        }
    }
}

/// The generated paragraphs. Sentences and words come back as a single paragraph.
pub fn generate_lorem<R: Rng + ?Sized>(spec: LoremSpec, rng: &mut R) -> Vec<String> {
    let count = spec.count.clamp(1, spec.unit.max_count());
    let mut writer = LoremWriter {
        rng,
        should_open_with_lorem: spec.should_start_with_lorem,
    };
    match spec.unit {
        LoremUnit::Paragraphs => (0..count).map(|_| writer.paragraph()).collect(),
        LoremUnit::Sentences => vec![writer.sentences(count)],
        LoremUnit::Words => vec![writer.words(count)],
    }
}

/// Wraps each paragraph in `<p>` tags.
pub fn format_as_html(paragraphs: &[String]) -> String {
    paragraphs
        .iter()
        .map(|paragraph| format!("<p>{paragraph}</p>"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_as_text(paragraphs: &[String]) -> String {
    paragraphs.join("\n\n")
}

struct LoremWriter<'a, R: Rng + ?Sized> {
    rng: &'a mut R,
    /// True until the opening words have been used.
    should_open_with_lorem: bool,
}

impl<R: Rng + ?Sized> LoremWriter<'_, R> {
    fn paragraph(&mut self) -> String {
        let sentence_count = self.rng.random_range(PARAGRAPH_SENTENCES);
        self.sentences(sentence_count)
    }

    fn sentences(&mut self, count: usize) -> String {
        (0..count)
            .map(|_| {
                let mut word_count = self.rng.random_range(SENTENCE_WORDS);
                if self.should_open_with_lorem {
                    word_count = word_count.max(OPENING.len());
                }
                let mut sentence = self.word_list(word_count);
                sentence.push('.');
                sentence
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// `count` words as one capitalised phrase with no closing full stop.
    fn words(&mut self, count: usize) -> String {
        self.word_list(count)
    }

    fn word_list(&mut self, count: usize) -> String {
        let mut words: Vec<String> = Vec::with_capacity(count);
        if self.should_open_with_lorem {
            self.should_open_with_lorem = false;
            words.extend(OPENING.iter().take(count).map(|word| (*word).to_owned()));
        }
        while words.len() < count {
            let word = WORDS.choose(&mut *self.rng).copied().unwrap_or("lorem");
            let is_last = words.len() + 1 == count;
            if !is_last && self.rng.random_bool(COMMA_CHANCE) {
                words.push(format!("{word},"));
            } else {
                words.push(word.to_owned());
            }
        }
        let mut phrase = words.join(" ");
        // A sentence never ends on a comma, even when the opening words are cut short.
        if phrase.ends_with(',') {
            phrase.pop();
        }
        capitalise_first(&phrase)
    }
}

fn capitalise_first(text: &str) -> String {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(characters).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn generate(unit: LoremUnit, count: usize, should_start_with_lorem: bool) -> Vec<String> {
        let spec = LoremSpec {
            unit,
            count,
            should_start_with_lorem,
        };
        generate_lorem(spec, &mut StdRng::seed_from_u64(7))
    }

    fn count_words(text: &str) -> usize {
        text.split_whitespace().count()
    }

    #[test]
    fn paragraphs_open_with_the_classic_phrase() {
        let paragraphs = generate(LoremUnit::Paragraphs, 3, true);
        assert_eq!(paragraphs.len(), 3);
        assert!(
            paragraphs[0].starts_with("Lorem ipsum dolor sit amet, consectetur adipiscing elit")
        );
        assert!(!paragraphs[1].starts_with("Lorem ipsum"));
        assert!(paragraphs.iter().all(|paragraph| paragraph.ends_with('.')));
    }

    #[test]
    fn sentences_come_as_one_paragraph_of_the_right_length() {
        let paragraphs = generate(LoremUnit::Sentences, 4, false);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].matches('.').count(), 4);
        assert!(!paragraphs[0].starts_with("Lorem"));
        assert!(paragraphs[0].starts_with(char::is_uppercase));
    }

    #[test]
    fn words_give_exactly_the_count_asked_for() {
        let paragraphs = generate(LoremUnit::Words, 25, true);
        assert_eq!(count_words(&paragraphs[0]), 25);
        let few = generate(LoremUnit::Words, 5, true);
        assert_eq!(few[0], "Lorem ipsum dolor sit amet");
    }

    #[test]
    fn counts_are_clamped() {
        assert_eq!(generate(LoremUnit::Paragraphs, 0, true).len(), 1);
        assert_eq!(
            generate(LoremUnit::Paragraphs, MAX_PARAGRAPHS + 50, true).len(),
            MAX_PARAGRAPHS
        );
    }

    #[test]
    fn formats_as_text_or_html() {
        let paragraphs = vec!["One.".to_owned(), "Two.".to_owned()];
        assert_eq!(format_as_text(&paragraphs), "One.\n\nTwo.");
        assert_eq!(format_as_html(&paragraphs), "<p>One.</p>\n<p>Two.</p>");
    }
}

use std::ops::Range;

/// Replace all recognized `left_delim placeholder right_delim` sequences with `replace_with`.
///
/// See [`first_placeholder_range()`] for how and when such a sequence is recognized.
pub fn replace_all_placeholders(
    string: &mut String,
    placeholder: &str,
    replace_with: &str,
    left_delim: char,
    right_delim: char,
) {
    let iter = StrPlaceholderRangeIter::new(placeholder, left_delim, right_delim);

    let mut next_range = 0..string.len();
    while let Some(range) = iter.next_range(&string[next_range.clone()]) {
        let last_start = next_range.start;
        let range = (range.start + last_start)..(range.end + last_start);

        string.replace_range(range.clone(), replace_with);
        next_range = (range.start + replace_with.len())..string.len();
    }
}

/// Get the first range into `string` that contains the sequence
/// `left_delim placeholder right_delim`.
///
/// For the left and right delimiters to be recognized, there must be:
/// - only one character that is equal to the delimiter
/// - an odd amount of the delimiter characters
///
/// This behaviour allows the delimiters/placeholder to be escaped.
pub fn first_placeholder_range(
    string: &str,
    placeholder: &str,
    left_delim: char,
    right_delim: char,
) -> Option<Range<usize>> {
    StrPlaceholderRangeIter::new(placeholder, left_delim, right_delim).next_range(string)
}

struct StrPlaceholderRangeIter<'a> {
    placeholder: &'a str,
    left_delim: char,
    left_delim_len: usize,
    right_delim: char,
    right_delim_len: usize,
}

impl<'a> StrPlaceholderRangeIter<'a> {
    pub fn new(placeholder: &'a str, left_delim: char, right_delim: char) -> Self {
        StrPlaceholderRangeIter {
            placeholder,
            left_delim,
            left_delim_len: left_delim.len_utf8(),
            right_delim,
            right_delim_len: right_delim.len_utf8(),
        }
    }

    pub fn next_range(&self, mut string: &str) -> Option<Range<usize>> {
        let mut offset = 0;
        loop {
            if let Some(start_index) = string.find(self.placeholder) {
                // check that on the left side of the placeholder are an odd number of
                // `self.left_delim` characters
                let is_left_delim = is_delimited(
                    string[..start_index]
                        .chars()
                        .rev()
                        .take_while(|c| *c == self.left_delim)
                        .count(),
                );
                let end_index = start_index + self.placeholder.len();

                // check that on the right side of the placeholder are an odd number of
                // `self.right_delim` characters
                let is_right_delim = is_delimited(
                    string[end_index..]
                        .chars()
                        .take_while(|c| *c == self.right_delim)
                        .count(),
                );

                if is_left_delim && is_right_delim {
                    break Some(
                        (start_index - self.left_delim_len + offset)
                            ..(end_index + self.right_delim_len + offset),
                    );
                } else {
                    offset += end_index;
                    string = &string[end_index..];
                }
            } else {
                break None;
            }
        }
    }
}

#[inline(always)]
pub fn is_delimited(n: usize) -> bool {
    // there must be at least one delimiter, and if more than one, the number must be odd
    n > 0 && (n % 2 != 0)
}

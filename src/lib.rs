use std::{
    hash::{Hash, Hasher},
    ops::Not,
};

use lsp_types::{Edit, TextEdit};
use rustc_hash::FxHashSet;

use crate::snippet::Snippet;

mod snippet;

type HashSet<T> = FxHashSet<T>;

#[derive(PartialEq, Eq)]
struct HashedEdit(TextEdit);

impl Hash for HashedEdit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.range.hash(state);
        self.0.new_text.hash(state);
    }
}

#[inline]
pub fn bench_modified(edits: Vec<Vec<(Edit, bool)>>) {
    edits.into_iter().for_each(|edit| {
        let (edits, snippet_edits) = edit.into_iter().fold(
            (HashSet::default(), HashSet::default()),
            |(mut edits, mut snippet_edits), (edit, is_active_entry)| {
                match edit {
                    Edit::Plain(edit) => {
                        let edit = HashedEdit(edit);
                        (edits.contains(&edit)).not().then(|| edits.insert(edit));
                    }
                    Edit::Annotated(edit) => {
                        let edit = HashedEdit(edit.text_edit);
                        (edits.contains(&edit)).not().then(|| edits.insert(edit));
                    }
                    Edit::Snippet(edit) => {
                        match Snippet::parse(&edit.snippet.value) {
                            Ok(snippet) if is_active_entry => {
                                snippet_edits.insert((edit.range, snippet));
                            }
                            // Since this buffer is not focused, apply a normal
                            // edit.
                            Ok(snippet) => {
                                let new_edit = HashedEdit(TextEdit {
                                    range: edit.range,
                                    new_text: snippet.text,
                                });
                                (edits.contains(&new_edit))
                                    .not()
                                    .then(|| edits.insert(new_edit));
                            }
                            _ => (),
                        }
                    }
                }

                (edits, snippet_edits)
            },
        );
    });
}

#[inline]
pub fn bench_original(edits: Vec<Vec<(Edit, bool)>>) {
    edits.into_iter().for_each(|edit| {
        let (mut edits, mut snippet_edits) = (vec![], vec![]);

        for (edit, is_active_entry) in edit {
            match edit {
                Edit::Plain(edit) => {
                    if !edits.contains(&edit) {
                        edits.push(edit)
                    }
                }
                Edit::Annotated(edit) => {
                    if !edits.contains(&edit.text_edit) {
                        edits.push(edit.text_edit)
                    }
                }
                Edit::Snippet(edit) => {
                    let Ok(snippet) = Snippet::parse(&edit.snippet.value) else {
                        continue;
                    };

                    if is_active_entry {
                        snippet_edits.push((edit.range, snippet));
                    } else {
                        // Since this buffer is not focused, apply a normal
                        // edit.
                        let new_edit = TextEdit {
                            range: edit.range,
                            new_text: snippet.text,
                        };
                        if !edits.contains(&new_edit) {
                            edits.push(new_edit);
                        }
                    }
                }
            }
        }
    });
}

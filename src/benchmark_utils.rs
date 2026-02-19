use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use gix::{
    Blob, ObjectId, Repository, Tree, bstr::BString, diff::tree_with_rewrites::Change,
    progress::Discard,
};
use imara_diff::{Algorithm, Diff, InternedInput};
use lsp_types::{
    AnnotatedTextEdit, Edit, Position, Range, SnippetTextEdit, StringValue, StringValueKind,
    TextEdit,
};
use tempfile::TempDir;

#[expect(
    clippy::must_use_candidate,
    reason = "Not using the return function is not a bug."
)]
pub fn produce_edits() -> Vec<(Vec<Edit>, bool)> {
    // 1. Get two revisions from the Zed worktree.
    // 2. Diff the revisions.
    // 3. Parse the contents of the revisions into an intermediate
    //    representation that still contains all git-specifics but appends into
    //    a two-tuple the git info and a boolean indicating whether the affected
    //    file/index in the worktree is deemed the active entry.
    // 4. Load into memory the contents of affected files during diffing for
    //    both revisions.
    // 5. Diff each pair of file buffers and produce one final representation
    //    with the results of the diff and the `active_entry` flag.
    // 6. Parse this representation into `lsp_types::Edit`s.
    // 7. Return the container of `lsp_types::Edit`s.
    let repo_path = new_repo_path();
    let repo = clone_repo(repo_path.path());
    let (head, rev) = get_commits(&repo);
    let revs_diff = diff_revisions(&repo, &head, &rev);
    let changes_ir = parse_ir(revs_diff);
    let blobs = into_blobs(&changes_ir, &head, &rev);
    let diff = diff_blobs(blobs);
    let edits = into_edits(diff);

    panic!("reached end of current work");
}

fn new_repo_path() -> TempDir {
    tempfile::tempdir().expect("default tmpdir should be valid")
}

fn clone_repo(repo_path: &Path) -> Repository {
    gix::prepare_clone("https://github.com/zed-industries/zed.git", repo_path)
        .expect("repo should be valid")
        .fetch_then_checkout(Discard, &AtomicBool::new(false))
        .expect("repo fetch should be infallible")
        .0
        .main_worktree(Discard, &AtomicBool::new(false))
        .expect("repo checkout should be infallible")
        .0
}

fn get_commits(repo: &Repository) -> (Tree<'_>, Tree<'_>) {
    (
        repo.head_tree().expect("repo head should be available"),
        repo.find_tree(
            repo.find_object(
                ObjectId::from_hex(b"da2d4ca5d9373dcfc1812125b262ae13769c35b8")
                    .expect("repo commit sha sourced directly from remote should be valid"),
            )
            .expect("repo commit is currently a valid object; maybe something chnaged on the zed github side")
            .peel_to_tree()
            .expect("repo commit is currently valid and bound to a tree revision")
            .id,
        )
        .expect("tree from repo commit sha sourced directly from remote should be valid"),
    )
}

fn diff_revisions(repo: &Repository, head: &Tree, other: &Tree) -> Vec<Change> {
    repo.diff_tree_to_tree(other, head, None)
        .expect("diffing should be possible")
}

fn parse_ir(changes: Vec<Change>) -> Vec<(BString, bool)> {
    let len = changes.len();

    changes
        .into_iter()
        .fold(
            (Vec::with_capacity(len), None),
            |(mut output, mut active_entry), change| {
                match change {
                    Change::Modification {
                        location,
                        previous_entry_mode,
                        entry_mode,
                        ..
                    } if active_entry.is_none()
                        && previous_entry_mode.eq(&entry_mode)
                        && entry_mode.is_blob() =>
                    {
                        active_entry = Some(location.clone());
                        output.push((location, true));
                    }
                    Change::Modification {
                        location,
                        previous_entry_mode,
                        entry_mode,
                        ..
                    } if let Some(active_entry) = &active_entry
                        && location.eq(active_entry)
                        && previous_entry_mode.eq(&entry_mode)
                        && entry_mode.is_blob() =>
                    {
                        output.push((location, true));
                    }
                    Change::Modification {
                        location,
                        previous_entry_mode,
                        entry_mode,
                        ..
                    } if previous_entry_mode.eq(&entry_mode) && entry_mode.is_blob() => {
                        output.push((location, false));
                    }
                    _ => (),
                }

                (output, active_entry)
            },
        )
        .0
}

fn into_blobs<'a>(
    diff: &[(BString, bool)],
    head: &Tree<'a>,
    rev: &Tree<'a>,
) -> Vec<(Blob<'a>, Blob<'a>, bool)> {
    let (head_blobs, rev_blobs) = diff.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut head_files, mut rev_files), (path, is_active_entry)| {
            let path = PathBuf::from(path.to_string());
            let path_error = path.display();
            let (error1, error2, error3) = (
                format!("failed to fetch head entry: {path_error}"),
                format!("entry should exist: {path_error}"),
                format!("entry object should exist: {path_error}"),
            );
            let (head_entry, rev_entry) = (
                head.lookup_entry_by_path(&path)
                    .expect(&error1)
                    .expect(&error2)
                    .object()
                    .expect(&error3)
                    .into_blob(),
                rev.lookup_entry_by_path(&path)
                    .expect(&error1)
                    .expect(&error2)
                    .object()
                    .expect(&error3)
                    .into_blob(),
            );

            head_files.push((head_entry, *is_active_entry));
            rev_files.push(rev_entry);

            (head_files, rev_files)
        },
    );

    head_blobs
        .into_iter()
        .zip(rev_blobs)
        .map(|((head_blob, is_active_entry), rev_blob)| (head_blob, rev_blob, is_active_entry))
        .collect()
}

fn diff_blobs(blobs: Vec<(Blob, Blob, bool)>) -> Vec<(String, Diff, bool)> {
    let len = blobs.len();

    blobs.into_iter().fold(
        Vec::with_capacity(len),
        |mut container, (mut head_blob, mut rev_blob, is_active_entry)| {
            let (head_string, rev_string) = (
                BString::new(head_blob.take_data()).to_string(),
                BString::new(rev_blob.take_data()).to_string(),
            );
            let interner = InternedInput::new(
                imara_diff::sources::lines(&rev_string),
                imara_diff::sources::lines(&head_string),
            );
            let mut diff = Diff::compute(Algorithm::Histogram, &interner);

            diff.postprocess_lines(&interner);
            container.push((head_string, diff, is_active_entry));

            container
        },
    )
}

fn into_edits(diff: Vec<(String, Diff, bool)>) -> Vec<(Vec<Edit>, bool)> {
    enum Choice {
        Plain,
        Annotated,
        Snippet,
    }

    impl Choice {
        fn new(input: usize) -> Option<Self> {
            match input {
                0 => Some(Self::Plain),
                1 => Some(Self::Annotated),
                2 => Some(Self::Snippet),
                _ => None,
            }
        }
    }

    let len = diff.len();

    diff.into_iter().fold(
        Vec::with_capacity(len),
        |mut container, (head_string, diff, is_active_entry)| {
            let hunks: Vec<_> = diff
                .hunks()
                .map(|hunk| {
                    let head_range = hunk.after;
                    let head_string = head_string
                        .lines()
                        .enumerate()
                        .skip_while(|(line_num, _)| line_num.lt(&(head_range.start as usize)))
                        .take_while(|(line_num, _)| line_num.lt(&(head_range.end as usize)))
                        .map(|(_, string)| string)
                        .collect::<String>();
                    let edit_choice =
                        Choice::new(todo!("Get a random number in the range 0-2.")).unwrap();

                    match edit_choice {
                        Choice::Plain => Edit::Plain(TextEdit {
                            range: Range::new(
                                Position::new(head_range.start, u32::MIN),
                                Position::new(head_range.end, u32::MAX),
                            ),
                            new_text: head_string,
                        }),
                        Choice::Annotated => Edit::Annotated(AnnotatedTextEdit {
                            text_edit: TextEdit {
                                range: Range::new(
                                    Position::new(head_range.start, u32::MIN),
                                    Position::new(head_range.end, u32::MAX),
                                ),
                                new_text: head_string,
                            },
                            annotation_id: String::new(),
                        }),
                        Choice::Snippet => Edit::Snippet(SnippetTextEdit {
                            range: Range::new(
                                Position::new(head_range.start, u32::MIN),
                                Position::new(head_range.end, u32::MAX),
                            ),
                            snippet: StringValue {
                                kind: StringValueKind::Snippet,
                                value: head_string,
                            },
                            annotation_id: None,
                        }),
                    }
                })
                .collect();

            container.push((hunks, is_active_entry));

            container
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::produce_edits;

    #[test]
    #[should_panic = "reached end"]
    fn it_works() {
        produce_edits();
    }

    #[test]
    #[ignore = "Only used for the purposes of inspecting an in-progress git-clone operation."]
    fn test_loc() {
        eprintln!("{}", tempfile::env::temp_dir().display());
    }
}

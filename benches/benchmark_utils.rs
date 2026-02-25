use std::{
    path::{Path, PathBuf},
    result::Result,
    sync::atomic::AtomicBool,
};

use gix::{
    Blob, Repository, Tree, bstr::BString, diff::tree_with_rewrites::Change, progress::Discard,
};
use imara_diff::{Algorithm, Diff, InternedInput};
use lsp_types::{
    AnnotatedTextEdit, Edit, Position, Range, SnippetTextEdit, StringValue, StringValueKind,
    TextEdit,
};
use rand::{
    RngExt,
    distr::{Distribution, Uniform},
    rngs::ThreadRng,
};
use tempfile::TempDir;

const ZED_REPO: &str = "https://github.com/zed-industries/zed.git";

#[expect(
    clippy::must_use_candidate,
    reason = "Not using the return value is not a bug."
)]
#[expect(
    clippy::missing_panics_doc,
    reason = "It's a personal benchmark so I just know what's going on."
)]
pub fn new_repo_path() -> TempDir {
    tempfile::tempdir().expect("default tmpdir should be valid")
}

#[expect(
    clippy::must_use_candidate,
    reason = "Not using the return value is not a bug."
)]
#[expect(
    clippy::missing_panics_doc,
    reason = "It's a personal benchmark so I just know what's going on."
)]
pub fn clone_repo(repo_path: &Path) -> Repository {
    gix::prepare_clone(ZED_REPO, repo_path)
        .expect("repo should be valid")
        .fetch_then_checkout(Discard, &AtomicBool::new(false))
        .expect("repo fetch should be infallible")
        .0
        .main_worktree(Discard, &AtomicBool::new(false))
        .expect("repo checkout should be infallible")
        .0
}

pub fn produce_edits(repo: &Repository) -> Vec<(Vec<Edit>, bool)> {
    let mut rng = rand::rng();

    let rev = get_rev_tree(repo, &mut rng);
    let head = get_head_tree(repo);
    let revs_diff = diff_revisions(repo, &head, &rev);
    let changes_ir = parse_ir(revs_diff);
    let blobs = into_blobs(&changes_ir, &head, &rev);
    let diff = diff_blobs(blobs);

    into_edits(diff, &mut rng)
}

fn get_rev_tree<'a>(repo: &'a Repository, rng: &mut ThreadRng) -> Tree<'a> {
    let commit = repo
        .references()
        .expect("the repo should have valid references")
        .prefixed("refs/heads/main")
        .expect("the repo shouldn't have invalid (pseudo) references")
        .find_map(|e| e.ok().iter_mut().find_map(|e| e.peel_to_commit().ok()))
        .expect("the head ref should point to the head of the main branch");
    let mut commits: Vec<_> = commit
        .ancestors()
        .all()
        .expect("this head ref should point to the parent commit and so on in the main branch")
        .filter_map(Result::ok)
        .collect();

    debug_assert!(commits.len() > 100);

    // We pick among the first commits because there's larger chances of finding
    // pure modifications (which are the only type of git change we consider for
    // this benchmark,) rather than deletions, rewrites, or additions.
    commits
        .remove(rng.random_range(0..100))
        .object()
        .expect("converting back into the odb object should be infallible")
        .tree()
        .expect("the selected commit should point to a valid tree in the repo")
}

fn get_head_tree(repo: &Repository) -> Tree<'_> {
    repo.head_tree().expect("repo head should be available")
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
                        && {
                            // To avoid symlinks.
                            previous_entry_mode.eq(&entry_mode)
                        }
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

fn into_edits(diff: Vec<(String, Diff, bool)>, rng: &mut ThreadRng) -> Vec<(Vec<Edit>, bool)> {
    #[derive(Debug)]
    enum EditChoice {
        Plain,
        Annotated,
        Snippet,
    }

    impl EditChoice {
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
    let distr = Uniform::try_from(0..3)
        .expect("shouldn't fail because the range is const and not ill-formed");

    diff.into_iter().fold(
        Vec::with_capacity(len),
        |mut container, (head_string, diff, is_active_entry)| {
            let edits: Vec<_> = diff
                .hunks()
                .map(|hunk| {
                    let head_range = hunk.after;
                    let head_string = head_string
                        .lines()
                        .enumerate()
                        .skip_while(|(line_num, _)| *line_num < head_range.start as usize)
                        .take_while(|(line_num, _)| *line_num < head_range.end as usize)
                        .map(|(_, string)| string)
                        .collect::<String>();
                    let edit_choice = EditChoice::new(distr.sample(rng))
                        .expect("the distribution should have produced a number within bounds");

                    match edit_choice {
                        EditChoice::Plain => Edit::Plain(TextEdit {
                            range: Range::new(
                                Position::new(head_range.start, u32::MIN),
                                Position::new(head_range.end, u32::MAX),
                            ),
                            new_text: head_string,
                        }),
                        EditChoice::Annotated => Edit::Annotated(AnnotatedTextEdit {
                            text_edit: TextEdit {
                                range: Range::new(
                                    Position::new(head_range.start, u32::MIN),
                                    Position::new(head_range.end, u32::MAX),
                                ),
                                new_text: head_string,
                            },
                            annotation_id: String::new(),
                        }),
                        EditChoice::Snippet => Edit::Snippet(SnippetTextEdit {
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

            container.push((edits, is_active_entry));

            container
        },
    )
}

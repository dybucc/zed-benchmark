use gix::{ObjectId, Repository, Tree, bstr::BString, diff::tree_with_rewrites::Change};
use lsp_types::Edit;

pub fn produce_edits() -> Vec<Vec<(Edit, bool)>> {
    // 1. Get two revisions in the Zed worktree.
    // 2. Diff the revisions.
    // 3. Parse the contents of the revisions into an intermediate
    //    representation that still contains all git-specifics but appends into
    //    a two-tuple the git info and a boolean indicating whether the affected
    //    file/index in the worktree is deemed the active entry.
    // 4. Load into memory the contents of affected files during diffing for
    //    both revisions.
    // 5. Parse into another intermediate representation each of the file
    //    buffers alongside the `active_entry` flag, and the change information
    //    in a three-element tuple.
    // 6. Parse into another intermediate representation each of the matching
    //    files (those with the same `location` component in the change-element
    //    of the tuple) into a single three-tuple with the buffers of each
    //    revision and the `active_entry` flag.
    // 7. Diff each pair of file buffers and produce one final representation
    //    with the results of the diff and the `active_entry` flag.
    //    This step may require extending the representation to also hold the
    //    underlying buffers if the information carried in the diff results
    //    proves to be insufficient.
    // 8. Parse this representation into `lsp_types::Edit`s.
    // 9. Return the container of `lsp_types::Edit`s.
    let repo_path = tempfile::tempdir().expect("default tmpdir should be valid");
    let repo = gix::prepare_clone(
        "https://github.com/zed-industries/zed.git",
        repo_path.path(),
    )
    .expect("repo should be valid")
    .fetch_then_checkout(progress, should_interrupt);
    let (head, rev) = get_commits(&repo);
    let revs_diff = diff_revisions(&repo, &head, &rev);
    let changes_ir = parse_ir(revs_diff);

    eprintln!("reached end of current work");

    todo!()
}

fn get_commits(repo: &'_ Repository) -> (Tree<'_>, Tree<'_>) {
    (
        repo.head_tree().expect("repo head should be available"),
        repo.find_tree(
            ObjectId::from_hex(b"da2d4ca5d9373dcfc1812125b262ae13769c35b8")
                .expect("repo commit sha sourced directly from remote should be valid"),
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
                    Change::Modification { location, .. } if active_entry.is_none() => {
                        active_entry = Some(location.clone());
                        output.push((location, true));
                    }
                    Change::Modification { location, .. }
                        if location == *active_entry.as_ref().unwrap() =>
                    {
                        output.push((location, true));
                    }
                    Change::Modification { location, .. } => output.push((location, false)),
                    _ => (),
                }

                (output, active_entry)
            },
        )
        .0
}

fn into_files() {}

fn parse_file_ir() {}

fn parse_matches_ir() {}

fn parse_diff_ir() {}

fn into_edits() {}

#[cfg(test)]
mod tests {
    use crate::produce_edits;

    #[test]
    fn it_works() {
        produce_edits();
    }
}

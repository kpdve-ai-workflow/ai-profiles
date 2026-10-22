use crate::wordpress::preflight::inside;

#[test]
fn project_relative_paths_reject_symlink_escape() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, temp.path().join("linked")).unwrap();
    assert!(inside(temp.path(), "linked/config.yaml").is_err());
}
#[test]
fn installed_wordpress_helper_context_is_selected_by_executable() {
    use crate::{layout::Layout, model::Client, wordpress::ddev::scripts_from_executable};
    let project = std::path::Path::new("/project");
    for client in Client::ALL {
        let scripts = Layout::Isolated(client).scripts();
        assert_eq!(
            scripts_from_executable(project, &project.join(&scripts).join("restore-updraft")),
            Some(scripts)
        );
    }
    assert!(
        scripts_from_executable(project, std::path::Path::new("/unrelated/restore-updraft"))
            .is_none()
    );
}

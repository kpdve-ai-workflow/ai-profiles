use crate::wordpress::{
    mapping::mappings, SiteMode, UrlMappings, WordPressSite, WordPressTopology,
};

#[test]
fn mapping_constructor_requires_explicit_non_chained_origins() {
    let chained = vec![
        "https://a.example=https://b.example".to_owned(),
        "https://b.example=https://c.example".to_owned(),
    ];
    assert!(mappings(&chained).is_err());
    assert_eq!(UrlMappings::default().entries().len(), 0);
    let _topology = WordPressTopology {
        sites: vec![WordPressSite {
            id: "1".into(),
            domain: String::new(),
            path: "/".into(),
            options_table: "wp_options".into(),
            home: "https://a.example".into(),
            site_url: "https://a.example".into(),
        }],
        networks: Vec::new(),
        site_mode: SiteMode::SingleSite,
    };
}

#[test]
fn all_site_urls_are_validated_before_any_process_call() {
    use std::{
        os::unix::process::ExitStatusExt,
        path::Path,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use crate::{
        error::{ProcessError, WordPressError},
        process::{CancellationToken, CommandOutput, CommandSpec, ProcessRunner},
        wordpress::{ddev::Ddev, localize::localize},
    };

    struct CountingRunner(Arc<AtomicUsize>);
    impl ProcessRunner for CountingRunner {
        fn run(&self, _spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CommandOutput {
                status: std::process::ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let runner = CountingRunner(Arc::clone(&calls));
    let ddev = Ddev::new(Path::new("/tmp/project"), &runner, CancellationToken::new());
    let mapping = mappings(&["https://source.example=https://local.example".to_owned()]).unwrap();
    let imported = WordPressTopology {
        sites: vec![WordPressSite {
            id: "1".to_owned(),
            domain: String::new(),
            path: "/".to_owned(),
            options_table: "wp_options".to_owned(),
            home: "https://unmapped.example".to_owned(),
            site_url: "https://local.example".to_owned(),
        }],
        networks: Vec::new(),
        site_mode: SiteMode::SingleSite,
    };
    let result = localize(
        &ddev,
        &crate::wordpress::TablePrefix::from_validated("wp_".to_owned()),
        &mapping,
        &imported,
    );
    assert!(matches!(result, Err(WordPressError::UnmappedUrl)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

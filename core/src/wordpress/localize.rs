use std::collections::HashMap;

use crate::{
    error::WordPressError,
    wordpress::{
        ddev::{literal, topology, Ddev},
        mapping::mapped_url,
        SiteMode, TablePrefix, UrlMappings, WordPressTopology,
    },
};

pub(crate) fn localize(
    ddev: &Ddev<'_>,
    prefix: &TablePrefix,
    mappings: &UrlMappings,
    imported: &WordPressTopology,
    operation_id: &str,
) -> Result<(), WordPressError> {
    let host_map = mappings
        .entries()
        .iter()
        .map(|mapping| (mapping.source.authority(), mapping.target.authority()))
        .collect::<HashMap<_, _>>();

    // Validate every site and network before the first query that changes data or bootstraps WP.
    if imported.sites.is_empty() {
        return Err(WordPressError::NoSitesToLocalize);
    }
    for site in &imported.sites {
        mapped_url(&site.home, mappings)?;
        mapped_url(&site.site_url, mappings)?;
        if imported.site_mode == SiteMode::Multisite && !host_map.contains_key(site.domain.as_str())
        {
            return Err(WordPressError::MissingBlogHostMapping);
        }
    }
    for network in &imported.networks {
        if !network.id.bytes().all(|byte| byte.is_ascii_digit())
            || !host_map.contains_key(network.domain.as_str())
        {
            return Err(WordPressError::InvalidNetworkMapping);
        }
    }
    tracing::info!(
        event = "restore.localization.preflight",
        operation_id,
        result = "passed",
        site_count = imported.sites.len(),
        network_count = imported.networks.len(),
        mapping_count = mappings.entries().len()
    );

    for site in &imported.sites {
        let home = mapped_url(&site.home, mappings)?;
        let site_url = mapped_url(&site.site_url, mappings)?;
        ddev.query(&format!(
            "UPDATE `{}` SET option_value={} WHERE option_name='home'",
            site.options_table,
            literal(&home)
        ))?;
        tracing::info!(
            event = "restore.localization.write",
            operation_id,
            target = "site_home",
            table = site.options_table,
            site_id = site.id
        );
        ddev.query(&format!(
            "UPDATE `{}` SET option_value={} WHERE option_name='siteurl'",
            site.options_table,
            literal(&site_url)
        ))?;
        tracing::info!(
            event = "restore.localization.write",
            operation_id,
            target = "site_url",
            table = site.options_table,
            site_id = site.id
        );
        if imported.site_mode == SiteMode::Multisite {
            let domain = host_map
                .get(site.domain.as_str())
                .ok_or(WordPressError::MissingBlogHostMapping)?;
            ddev.query(&format!(
                "UPDATE `{}blogs` SET domain={} WHERE blog_id={}",
                prefix.as_str(),
                literal(domain),
                site.id
            ))?;
            tracing::info!(
                event = "restore.localization.write",
                operation_id,
                target = "blog_domain",
                table = format!("{}blogs", prefix.as_str()),
                site_id = site.id
            );
        }
    }
    for network in &imported.networks {
        let domain = host_map
            .get(network.domain.as_str())
            .ok_or(WordPressError::InvalidNetworkMapping)?;
        ddev.query(&format!(
            "UPDATE `{}site` SET domain={} WHERE id={}",
            prefix.as_str(),
            literal(domain),
            network.id
        ))?;
        tracing::info!(
            event = "restore.localization.write",
            operation_id,
            target = "network_domain",
            table = format!("{}site", prefix.as_str()),
            network_id = network.id
        );
    }

    let target = mapped_url(&imported.sites[0].home, mappings)?;
    for mapping in mappings.entries() {
        let args = vec![
            format!("--url={target}"),
            "search-replace".to_owned(),
            mapping.source.as_str().to_owned(),
            mapping.target.as_str().to_owned(),
            "--all-tables-with-prefix".to_owned(),
            "--precise".to_owned(),
            "--skip-columns=guid".to_owned(),
            "--report-changed-only".to_owned(),
            "--format=count".to_owned(),
        ];
        let count = ddev.wp(&args)?;
        tracing::info!(event = "restore.localization.search_replace", operation_id, source = mapping.source.as_str(), target = mapping.target.as_str(), result = "executed", changed_rows = %count);
        let mut dry_run_args = args;
        dry_run_args.push("--dry-run".to_owned());
        let dry_run_count = ddev.wp(&dry_run_args)?;
        tracing::info!(event = "restore.localization.search_replace_verification", operation_id, source = mapping.source.as_str(), target = mapping.target.as_str(), remaining_matches = %dry_run_count);
        if count.is_empty()
            || !count.bytes().all(|byte| byte.is_ascii_digit())
            || dry_run_count != "0"
        {
            return Err(WordPressError::SearchReplaceIncomplete);
        }
    }

    for site in &imported.sites {
        ddev.query(&format!(
            "UPDATE `{}` SET option_value='0' WHERE option_name='blog_public'",
            site.options_table
        ))?;
        tracing::info!(
            event = "restore.localization.write",
            operation_id,
            target = "private_site_visibility",
            table = site.options_table,
            site_id = site.id,
            value = 0
        );
    }

    let final_topology = topology(ddev, prefix)?;
    if final_topology.sites.len() != imported.sites.len() {
        return Err(WordPressError::SiteCountChanged);
    }
    for (before, after) in imported.sites.iter().zip(&final_topology.sites) {
        if after.id != before.id
            || after.path != before.path
            || after.home != mapped_url(&before.home, mappings)?
            || after.site_url != mapped_url(&before.site_url, mappings)?
            || (imported.site_mode == SiteMode::Multisite
                && host_map.get(before.domain.as_str()).copied() != Some(after.domain.as_str()))
        {
            return Err(WordPressError::SiteVerificationFailed);
        }
    }
    let expected_networks = imported
        .networks
        .iter()
        .map(|network| {
            host_map
                .get(network.domain.as_str())
                .map(|domain| (network.id.as_str(), *domain, network.path.as_str()))
                .ok_or(WordPressError::InvalidNetworkMapping)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let actual_networks = final_topology
        .networks
        .iter()
        .map(|network| {
            (
                network.id.as_str(),
                network.domain.as_str(),
                network.path.as_str(),
            )
        })
        .collect::<Vec<_>>();
    if actual_networks != expected_networks {
        return Err(WordPressError::NetworkVerificationFailed);
    }
    ddev.wp(&[
        format!("--url={target}"),
        "core".to_owned(),
        "is-installed".to_owned(),
    ])?;
    tracing::info!(
        event = "restore.localization.verified",
        operation_id,
        site_count = final_topology.sites.len(),
        network_count = final_topology.networks.len(),
        result = "passed"
    );
    Ok(())
}

<?php
// Required by every WP-CLI operation of restore-updraft.py, before WP bootstrap.
if (!defined('WP_CLI') || !WP_CLI) {
    exit(1);
}
$ai_profile_local_guard = function () {
    $environment = defined('WP_ENVIRONMENT_TYPE') ? WP_ENVIRONMENT_TYPE : getenv('WP_ENVIRONMENT_TYPE');
    if (getenv('IS_DDEV_PROJECT') !== 'true' || $environment !== 'local') {
        WP_CLI::error('Import requires DDEV and WP_ENVIRONMENT_TYPE=local.');
    }
    if (!defined('DB_HOST') || !in_array(DB_HOST, array('db', 'db:3306'), true)
        || !defined('DB_NAME') || DB_NAME !== 'db') {
        WP_CLI::error('Import requires the local DDEV db service and database db.');
    }
    global $table_prefix;
    if (!is_string($table_prefix) || !preg_match('/^[A-Za-z0-9_]+$/D', $table_prefix)) {
        WP_CLI::error('Unsupported WordPress table prefix.');
    }
    $content = rtrim(ABSPATH, '/') . '/wp-content';
    if ((defined('WP_CONTENT_DIR') && WP_CONTENT_DIR !== $content)
        || (defined('WPMU_PLUGIN_DIR') && WPMU_PLUGIN_DIR !== $content . '/mu-plugins')) {
        WP_CLI::error('Custom content/mu-plugin paths are not supported by this importer.');
    }
};
WP_CLI::add_hook('after_wp_config_load', $ai_profile_local_guard);
// Unlike `wp db prefix`, this command does not bootstrap WordPress/plugins.
WP_CLI::add_command('ai-profile-local-prefix', function () use ($ai_profile_local_guard) {
    $ai_profile_local_guard();
    global $table_prefix;
    WP_CLI::line($table_prefix);
}, array('when' => 'after_wp_config_load'));

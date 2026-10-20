<?php
/** Plugin Name: AI Profile Local Import Safety */
// Inert outside DDEV; this is not a production security mechanism.
if (getenv('IS_DDEV_PROJECT') !== 'true') {
    return;
}
if (!defined('DISABLE_WP_CRON')) {
    define('DISABLE_WP_CRON', true);
}
add_filter('pre_wp_mail', '__return_true', PHP_INT_MAX);
add_filter('pre_get_ready_cron_jobs', '__return_empty_array', PHP_INT_MAX);
add_filter('pre_http_request', function () {
    return new WP_Error('ai_profile_local', 'Outbound WordPress HTTP is disabled after local import.');
}, PHP_INT_MAX);

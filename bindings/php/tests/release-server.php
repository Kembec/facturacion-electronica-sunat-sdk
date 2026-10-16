<?php

declare(strict_types=1);

// Router for `php -S`: serves the release tree in RELEASE_ROOT, and turns `/redirect/<host>/<rest>` into a
// 302 to `http://<host>/<rest>`, the way GitHub redirects a release asset to its storage host.

$path = (string) strtok($_SERVER['REQUEST_URI'], '?');

if (preg_match('#^/redirect/([^/]+)/(.*)$#', $path, $m) === 1) {
    header('Location: http://' . $m[1] . '/' . $m[2], true, 302);

    return true;
}

$file = getenv('RELEASE_ROOT') . $path;
if (strpos($path, '..') !== false || !is_file($file)) {
    http_response_code(404);

    return true;
}

header('Content-Type: application/octet-stream');
readfile($file);

return true;

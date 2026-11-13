<?php

declare(strict_types=1);

header('Content-Type: application/json');

echo json_encode([
    'app' => 'rakitray-php-node-starter',
    'php_version' => PHP_VERSION,
    'memory_limit' => ini_get('memory_limit'),
    'status' => 'ok',
], JSON_PRETTY_PRINT);


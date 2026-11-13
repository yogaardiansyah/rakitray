# Sample Project: PHP + Node Starter

## Quick Test

```bash
cd examples/php-node-starter

# 1. Install runtimes declared in .rakitray.toml
rakit install php@8.3.9 node@22.11.0

# 2. Validate config & port availability
rakit up

# 3. Activate isolated PHP & Node in current shell only
eval "$(rakit env)"
php -v
node -v

# 4. Serve the PHP endpoint
php -S 127.0.0.1:8080 -t public
```


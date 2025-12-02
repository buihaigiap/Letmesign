#!/bin/bash
# Deployment script for Letmesign production

set -e
echo "🚀 Deploying Letmesign..."

# Pull latest code
echo "📥 Pulling code..."
git pull origin main

# Install dependencies
echo "📦 Checking dependencies..."
which pdftoppm || sudo apt-get install -y poppler-utils
which convert || sudo apt-get install -y imagemagick

# Build
echo "🔨 Building..."
cargo build --release

# Restart
echo "🔄 Restarting service..."
if systemctl list-units | grep -q letmesign; then
    sudo systemctl restart letmesign
else
    pkill -f letmesign || true
    nohup cargo run --release > /var/log/letmesign.log 2>&1 &
fi

echo "✅ Deployment complete!"

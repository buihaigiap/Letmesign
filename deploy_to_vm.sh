#!/bin/bash
# Deploy to VM script

set -e

VM_HOST="192.168.90.37"
VM_USER="root"  # Try root user
VM_PATH="/opt/letmesign"

echo "🚀 Deploying to VM ${VM_HOST}..."

# Build locally first
echo "🔨 Building locally..."
cargo build --release

# Copy binary to VM
echo "📤 Copying binary to VM..."
scp target/release/letmesign ${VM_USER}@${VM_HOST}:${VM_PATH}/

# Restart service on VM
echo "🔄 Restarting service on VM..."
ssh ${VM_USER}@${VM_HOST} "systemctl restart letmesign || (pkill -f letmesign && nohup ${VM_PATH}/letmesign > /var/log/letmesign.log 2>&1 &)"

echo "✅ Deployment to VM complete!"

pub mod kwin;
pub mod theme;
pub mod performance;
MODOF

echo "✅ Created src/config/mod.rs"
cd /home/benji/AetherForge/apps/plasma-shell

# Create config directory
mkdir -p src/config

# Create mod.rs
cat > src/config/mod.rs << 'MODEOF'
pub mod kwin;
pub mod theme;
pub mod performance;

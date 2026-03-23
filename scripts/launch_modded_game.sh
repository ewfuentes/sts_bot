#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

STS_DIR="$HOME/Library/Application Support/Steam/steamapps/common/SlayTheSpire/SlayTheSpire.app/Contents/Resources"
WORKSHOP="$HOME/Library/Application Support/Steam/steamapps/workshop/content/646570"
MTS_JAR="$WORKSHOP/1605060445/ModTheSpire.jar"
COMMOD_CONFIG="$HOME/Library/Preferences/ModTheSpire/CommunicationMod/config.properties"
BRIDGE_SCRIPT="$PROJECT_DIR/glue/commod_bridge.py"
COMMOD_WORKSHOP_JAR="$WORKSHOP/2131373661/CommunicationMod.jar"
COMMOD_BUILT_JAR="$PROJECT_DIR/CommunicationMod/target/CommunicationMod.jar"

# Build our forked CommunicationMod
echo "Building CommunicationMod..."
JAVA_HOME="$(brew --prefix openjdk)" PATH="$(brew --prefix openjdk)/bin:$PATH" \
    mvn -f "$PROJECT_DIR/CommunicationMod/pom.xml" package -q

# Replace Workshop JAR with our build
cp "$COMMOD_BUILT_JAR" "$COMMOD_WORKSHOP_JAR"

# Ensure CommunicationMod is configured to launch our bridge
mkdir -p "$(dirname "$COMMOD_CONFIG")"
cat > "$COMMOD_CONFIG" <<EOF
#CommunicationMod config
command=python3 ${BRIDGE_SCRIPT}
runAtGameStart=true
EOF

echo "CommunicationMod configured to run: python3 $BRIDGE_SCRIPT"
echo "Socket will be at: /tmp/sts_commod.sock"
echo "Launching Slay the Spire..."

cd "$STS_DIR"
exec ./jre/bin/java -jar "$MTS_JAR" \
    --skip-launcher \
    --skip-intro \
    --mods basemod,stslib,CommunicationMod,superfastmode,BoardGame

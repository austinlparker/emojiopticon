# Configuration
SERVICE_NAME = emojiopticon
INSTALL_DIR = /opt/$(SERVICE_NAME)
CONFIG_DIR = /etc/$(SERVICE_NAME)
LOG_DIR = /var/log/$(SERVICE_NAME)
DATA_DIR = /var/lib/$(SERVICE_NAME)
SERVICE_USER = $(SERVICE_NAME)
SERVICE_GROUP = $(SERVICE_NAME)
BINARY_NAME = bsky-frequency-analyzer

# Commands
CARGO = cargo
SYSTEMCTL = systemctl
INSTALL = install
MKDIR = mkdir -p
CHOWN = chown
CHMOD = chmod

# Check if running as root
ifeq ($(shell id -u),0)
    SUDO =
else
    SUDO = sudo
endif

.PHONY: all build install clean uninstall restart status logs

all: build install

# Build the release binary
build:
	$(CARGO) build --release

# Install everything
install: create_dirs copy_files setup_service

create_dirs:
    $(SUDO) $(MKDIR) $(INSTALL_DIR)
    $(SUDO) $(MKDIR) $(CONFIG_DIR)
    $(SUDO) $(MKDIR) $(LOG_DIR)
    $(SUDO) $(MKDIR) $(DATA_DIR)
    # Create service user if it doesn't exist
    $(SUDO) id -u $(SERVICE_USER) &>/dev/null || $(SUDO) useradd -r -s /bin/false $(SERVICE_USER)

copy_files:
	$(SUDO) $(INSTALL) -m 755 target/release/$(BINARY_NAME) $(INSTALL_DIR)/$(SERVICE_NAME)
	$(SUDO) $(INSTALL) -m 644 config/prompts.toml $(CONFIG_DIR)/prompts.toml
	$(SUDO) $(INSTALL) -m 644 deploy/$(SERVICE_NAME).service /etc/systemd/system/
	$(SUDO) $(INSTALL) -m 644 deploy/$(SERVICE_NAME).logrotate /etc/logrotate.d/$(SERVICE_NAME)
	$(SUDO) $(CHOWN) -R $(SERVICE_USER):$(SERVICE_GROUP) $(INSTALL_DIR)
	$(SUDO) $(CHOWN) -R $(SERVICE_USER):$(SERVICE_GROUP) $(CONFIG_DIR)
	$(SUDO) $(CHOWN) -R $(SERVICE_USER):$(SERVICE_GROUP) $(LOG_DIR)
	$(SUDO) $(CHOWN) -R $(SERVICE_USER):$(SERVICE_GROUP) $(DATA_DIR)

setup_service:
	$(SUDO) $(SYSTEMCTL) daemon-reload
	$(SUDO) $(SYSTEMCTL) enable $(SERVICE_NAME)
	$(SUDO) $(SYSTEMCTL) restart $(SERVICE_NAME)

# Clean up built files
clean:
	$(CARGO) clean

# Uninstall everything
uninstall:
	$(SUDO) $(SYSTEMCTL) stop $(SERVICE_NAME)
	$(SUDO) $(SYSTEMCTL) disable $(SERVICE_NAME)
	$(SUDO) rm -f /etc/systemd/system/$(SERVICE_NAME).service
	$(SUDO) rm -f /etc/logrotate.d/$(SERVICE_NAME)
	$(SUDO) rm -rf $(INSTALL_DIR)
	$(SUDO) rm -rf $(CONFIG_DIR)
	$(SUDO) rm -rf $(LOG_DIR)
	$(SUDO) $(SYSTEMCTL) daemon-reload

# Restart the service
restart:
	$(SUDO) $(SYSTEMCTL) restart $(SERVICE_NAME)

# Check service status
status:
	$(SUDO) $(SYSTEMCTL) status $(SERVICE_NAME)

# View logs
logs:
	$(SUDO) journalctl -u $(SERVICE_NAME) -f

# Development helpers
dev-deps:
	$(SUDO) apt-get update
	$(SUDO) apt-get install -y build-essential pkg-config libssl-dev

# Configuration
SERVICE_NAME = emojiopticon
INSTALL_DIR = /opt/$(SERVICE_NAME)
CONFIG_DIR = /etc/$(SERVICE_NAME)
LOG_DIR = /var/log/$(SERVICE_NAME)
DATA_DIR = /var/lib/$(SERVICE_NAME)
SERVICE_USER = $(SERVICE_NAME)
SERVICE_GROUP = $(SERVICE_NAME)
BINARY_NAME = bsky-frequency-analyzer
PORT = 3000

# Try to get OPENAI_API_KEY from environment, or prompt for it
OPENAI_API_KEY ?= $(shell bash -c 'read -p "OpenAI API Key: " key; echo $$key')

# Commands
CARGO = cargo
SYSTEMCTL = systemctl
INSTALL = install
MKDIR = mkdir -p
CHOWN = chown
CHMOD = chmod
UFW = ufw

# Check if running as root
ifeq ($(shell id -u),0)
	SUDO =
else
	SUDO = sudo
endif

.PHONY: all build install clean uninstall restart status logs create_user create_dirs copy_files setup_service setup_env setup_firewall

all: build install

# Build the release binary
build:
	$(CARGO) build --release

# Install everything
install: create_user create_dirs setup_env copy_files setup_firewall setup_service

# Create service user first
create_user:
	$(SUDO) groupadd -f $(SERVICE_GROUP) || true
	$(SUDO) useradd -r -s /bin/false -g $(SERVICE_GROUP) $(SERVICE_USER) || true
	# Verify user exists
	$(SUDO) id $(SERVICE_USER)

create_dirs: create_user
	$(SUDO) $(MKDIR) $(INSTALL_DIR)
	$(SUDO) $(MKDIR) $(CONFIG_DIR)
	$(SUDO) $(MKDIR) $(LOG_DIR)
	$(SUDO) $(MKDIR) $(DATA_DIR)

setup_env:
	@echo "Setting up environment file..."
	$(SUDO) touch $(CONFIG_DIR)/environment
	@echo "OPENAI_API_KEY=$(OPENAI_API_KEY)" | $(SUDO) tee $(CONFIG_DIR)/environment > /dev/null
	$(SUDO) $(CHOWN) $(SERVICE_USER):$(SERVICE_GROUP) $(CONFIG_DIR)/environment
	$(SUDO) $(CHMOD) 600 $(CONFIG_DIR)/environment

setup_firewall:
	@echo "Configuring firewall..."
	$(SUDO) $(UFW) status | grep -q "Status: active" || (echo "Enabling UFW..." && $(SUDO) $(UFW) enable)
	$(SUDO) $(UFW) allow $(PORT)/tcp comment "$(SERVICE_NAME) web interface"
	$(SUDO) $(UFW) reload

copy_files: create_dirs
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
	-$(SUDO) $(SYSTEMCTL) stop $(SERVICE_NAME)
	-$(SUDO) $(SYSTEMCTL) disable $(SERVICE_NAME)
	-$(SUDO) rm -f /etc/systemd/system/$(SERVICE_NAME).service
	-$(SUDO) rm -f /etc/logrotate.d/$(SERVICE_NAME)
	-$(SUDO) rm -rf $(INSTALL_DIR)
	-$(SUDO) rm -rf $(CONFIG_DIR)
	-$(SUDO) rm -rf $(LOG_DIR)
	-$(SUDO) rm -rf $(DATA_DIR)
	-$(SUDO) userdel $(SERVICE_USER)
	-$(SUDO) groupdel $(SERVICE_GROUP)
	-$(SUDO) $(UFW) delete allow $(PORT)/tcp
	$(SUDO) $(SYSTEMCTL) daemon-reload

# Restart the service
restart:
	$(SUDO) $(SYSTEMCTL) restart $(SERVICE_NAME)

# Check status
status:
	$(SUDO) $(SYSTEMCTL) status $(SERVICE_NAME)
	@echo "\nFirewall status:"
	$(SUDO) $(UFW) status | grep $(PORT)

# View logs
logs:
	$(SUDO) journalctl -u $(SERVICE_NAME) -f

# Development helpers
dev-deps:
	$(SUDO) apt-get update
	$(SUDO) apt-get install -y build-essential pkg-config libssl-dev ufw

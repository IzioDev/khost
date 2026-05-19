use crate::imports::*;

const CONFIG_VERSION: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u64,
    // Bootstrap was executed
    pub bootstrap: bool,
    pub disable_sudo_prompt: bool,
    pub public: bool,
    pub fqdn: Option<Vec<String>>,
    pub ip: Option<String>,
    pub nginx: nginx::Config,
    pub kaspad: Vec<kaspad::Config>,
    pub resolver: resolver::Config,
}

impl Config {
    pub fn try_new() -> Result<Self> {
        let origin = Origin::try_new("https://github.com/aspectron/kaspa-resolver", None)?;
        let resolver = resolver::Config::new(origin)
            .with_stats()
            .with_local_interface(8989);

        let pnnv1_origin = Self::pnnv1_origin()?;
        let tn12_origin = Self::pnnv1_toccata_origin()?;

        let kaspad = SupportedNetwork::iter()
            .copied()
            .map(|network| {
                let selected_origin = match network {
                    SupportedNetwork::Mainnet | SupportedNetwork::Testnet10 => pnnv1_origin.clone(),
                    SupportedNetwork::Testnet12 => tn12_origin.clone(),
                };
                kaspad::Config::new(selected_origin, network.into())
            })
            .collect::<Vec<_>>();

        let nginx = nginx::Config::default();

        Ok(Config {
            version: CONFIG_VERSION,
            bootstrap: false,
            disable_sudo_prompt: false,
            public: true,
            fqdn: None,
            ip: None,
            nginx,
            kaspad,
            resolver,
        })
    }

    pub fn pnnv1_origin() -> Result<Origin> {
        Origin::try_new("https://github.com/aspectron/rusty-kaspa", Some("pnn-v1"))
    }

    pub fn pnnv1_toccata_origin() -> Result<Origin> {
        Origin::try_new(
            "https://github.com/aspectron/rusty-kaspa",
            Some("pnn-v1-toccata"),
        )
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = data_folder().join("config.json");
        if !config_path.exists() {
            return Err(Error::custom("Config file not found"));
        }
        let mut config: Config = serde_json::from_str(&fs::read_to_string(config_path)?)?;
        let last_version = config.version;

        let mut update = false;
        // v1 -> v2
        if last_version <= 1 {
            config.kaspad.iter_mut().for_each(|kaspad_config| {
                if kaspad_config.network().is_supported() {
                    if let Some(branch) = kaspad_config.origin_mut().branch_mut() {
                        if branch == "omega" {
                            *branch = "pnn-v1".to_string();
                            update = true;
                        }
                    }
                }
            });
        }

        // v2 -> v3
        if last_version <= 2 {
            // push tn12
            if !config
                .kaspad
                .iter()
                .any(|config| config.network() == SupportedNetwork::Testnet12.into())
            {
                config.kaspad.push(kaspad::Config::new(
                    Self::pnnv1_toccata_origin()?,
                    SupportedNetwork::Testnet12.into(),
                ));
                update = true;
            }

            // update rk origin
            for kaspad_config in config
                .kaspad
                .iter_mut()
                .filter(|kaspad_config| kaspad_config.is_supported_network())
            {
                match kaspad_config.network() {
                    Network::Supported(SupportedNetwork::Mainnet) => {
                        *kaspad_config.origin_mut() = Self::pnnv1_origin()?;
                        update = true;
                    }
                    // set to toccata in the meantime of master release
                    Network::Supported(SupportedNetwork::Testnet10) => {
                        *kaspad_config.origin_mut() = Self::pnnv1_toccata_origin()?;
                        update = true;

                        log::warning("warning: a manual tn10 node DB reset will be needed, either\n\t- remove manually, default location: `~/.rusty-kaspa/kaspa-testnet-10`\t- or, uninstall tn10 and re-install");
                    }
                    _ => (),
                }
            }
        }

        // keep until v3 is current, gate it to <= 3 on v4
        update |= config.remove_unused_legacy_network(DeprecatedNetwork::Testnet11);

        if config.version < CONFIG_VERSION {
            config.version = CONFIG_VERSION;
            update = true;
        }

        if update {
            log::success(format!(
                "Updated kHOST config to version {}",
                CONFIG_VERSION
            ))?;
            config.save()?;
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = data_folder().join("config.json");
        fs::write(config_path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    /// considered unused if no systemd service and no active in cfg
    fn remove_unused_legacy_network(&mut self, network: DeprecatedNetwork) -> bool {
        let network = Network::from(network);
        let kaspad_len = self.kaspad.len();
        self.kaspad.retain(|kaspad_config| {
            kaspad_config.network() != network
                || kaspad_config.is_enabled()
                || systemd::is_active(kaspad_config.service_name()).unwrap_or(true)
        });
        self.kaspad.len() != kaspad_len
    }

    pub fn reset() {
        let path = data_folder().join("config.json");
        if let Err(err) = fs::remove_file(&path) {
            let _ = log::error(format!(
                "Failed to reset config file: {}\n{err}",
                path.display()
            ));
        }
    }
}

pub fn fqdn<S: Display>(prompt: S) -> Result<String> {
    match cliclack::input(prompt)
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Please enter a valid domain name".to_string())
            } else if let Err(err) = addr::parse_domain_name(input) {
                Err(err.to_string())
            } else {
                Ok(())
            }
        })
        .interact::<String>()
    {
        Ok(fqdn) => Ok(fqdn.to_string()),
        Err(e) => Err(e.into()),
    }
}

pub fn public_network(ctx: &mut Context) -> Result<()> {
    ctx.config.public =
        confirm("Would you like this node to join the Kaspa public node network?").interact()?;
    if ctx.config.public {
        if let (Some(ip), Some(id)) = (ctx.config.ip.as_ref(), ctx.system.system_id.as_ref()) {
            cliclack::note(
                "Public Node Network",
                format!(
                    r#"
Thank you for contributing to the Kaspa ecosystem!

          Your system id is: {}
Your public IPv4 address is: {}

Please register your system via the following form:
-> https://forms.gle/mWemBbwNEjXsFC5F7
Please reach out to one of the public node maintainers
on Telegram or Kaspa Discord once you have submitted
the information using this form.
"#,
                    ::console::style(format!("{id:016x}")).yellow(),
                    ::console::style(ip).cyan()
                ),
            )?;
        } else {
            log::error("Unable to detect your public IPv4 address.\nPlease resolve this problem before continuing.")?;
            return Err(Error::custom("Unable to detect public ip :("));
        }
    } else {
        // TODO: validate space-separated fqdns
        let fqdns = fqdn("Enter fully qualified domain names (FQDN):")?;
        ctx.config.fqdn = Some(fqdns.split_whitespace().map(String::from).collect());
    }
    ctx.config.save()?;
    Ok(())
}

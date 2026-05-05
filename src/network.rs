use crate::imports::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "interface", content = "port")]
#[serde(rename_all = "lowercase")]
pub enum Interface {
    Public(u16),
    Local(u16),
}

impl Display for Interface {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Interface::Public(port) => write!(f, "0.0.0.0:{}", port),
            Interface::Local(port) => write!(f, "127.0.0.1:{}", port),
        }
    }
}

impl Interface {
    pub fn port(&self) -> u16 {
        match self {
            Interface::Public(port) => *port,
            Interface::Local(port) => *port,
        }
    }
}

#[derive(Default, Describe, Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SupportedNetwork {
    #[default]
    Mainnet,
    Testnet10,
    Testnet12,
}

#[derive(Describe, Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DeprecatedNetwork {
    Testnet11,
}

impl Display for SupportedNetwork {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SupportedNetwork::Mainnet => write!(f, "mainnet"),
            SupportedNetwork::Testnet10 => write!(f, "testnet-10"),
            SupportedNetwork::Testnet12 => write!(f, "testnet-12"),
        }
    }
}

impl Display for DeprecatedNetwork {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            DeprecatedNetwork::Testnet11 => write!(f, "testnet-11"),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum Network {
    Supported(SupportedNetwork),
    Deprecated(DeprecatedNetwork),
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Network::Supported(network) => write!(f, "{network}"),
            Network::Deprecated(network) => write!(f, "{network}"),
        }
    }
}

impl Network {
    pub fn supported() -> impl Iterator<Item = Network> {
        SupportedNetwork::iter().copied().map(Self::from)
    }

    pub fn deprecated() -> impl Iterator<Item = Network> {
        DeprecatedNetwork::iter().copied().map(Self::from)
    }

    pub fn is_supported(self) -> bool {
        matches!(self, Network::Supported(_))
    }

    pub fn is_deprecated(self) -> bool {
        matches!(self, Network::Deprecated(_))
    }
}

impl Default for Network {
    fn default() -> Self {
        SupportedNetwork::default().into()
    }
}

impl From<SupportedNetwork> for Network {
    fn from(network: SupportedNetwork) -> Self {
        Self::Supported(network)
    }
}

impl From<DeprecatedNetwork> for Network {
    fn from(network: DeprecatedNetwork) -> Self {
        Self::Deprecated(network)
    }
}

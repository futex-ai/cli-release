use cli_release_interface::ReleaseAssetInfo;

use super::GitHubAsset;
use crate::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AssetTemplate {
    template: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemplateField {
    Binary,
    Version,
    Target,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TemplatePart {
    Literal(String),
    Field(TemplateField),
}

impl AssetTemplate {
    pub(super) fn new(template: String) -> Result<Self> {
        let parts = template_parts(&template)?;
        if !parts
            .iter()
            .any(|part| matches!(part, TemplatePart::Field(TemplateField::Binary)))
            || !parts
                .iter()
                .any(|part| matches!(part, TemplatePart::Field(TemplateField::Version)))
            || !parts
                .iter()
                .any(|part| matches!(part, TemplatePart::Field(TemplateField::Target)))
        {
            return Err(Error::InvalidAssetTemplate { template });
        }
        Ok(Self { template })
    }

    pub(super) fn archive_name(&self, binary: &str, version: &str, target: &str) -> String {
        self.template
            .replace("{binary}", binary)
            .replace("{version}", version)
            .replace("{target}", target)
    }

    #[cfg(test)]
    pub(super) fn asset_info_from_asset(
        &self,
        asset: &GitHubAsset,
        version: &str,
    ) -> Result<Option<ReleaseAssetInfo>> {
        let Some(mut info) = self.asset_info_from_name(&asset.name, version, None)? else {
            return Ok(None);
        };
        info.sha256 = sha256_from_asset_digest(asset)?;
        Ok(Some(info))
    }

    pub(super) fn asset_info_from_name(
        &self,
        asset_name: &str,
        version: &str,
        sha256: Option<String>,
    ) -> Result<Option<ReleaseAssetInfo>> {
        let Some(parsed) = parse_asset_name(&self.template, asset_name, version)? else {
            return Ok(None);
        };
        Ok(Some(ReleaseAssetInfo {
            binary: parsed.binary.clone(),
            version: version.to_owned(),
            target: parsed.target.clone(),
            asset_name: asset_name.to_owned(),
            download_url: format!(
                "/releases/download/{}/{}/{}",
                parsed.binary, version, parsed.target
            ),
            sha256,
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedAsset {
    binary: String,
    target: String,
}

#[cfg(test)]
pub(super) fn legacy_asset_info_from_name(
    asset_name: &str,
    version: &str,
    sha256: Option<String>,
) -> Option<ReleaseAssetInfo> {
    let template = AssetTemplate::new("{binary}-{version}-{target}.tar.gz".to_owned()).ok()?;
    template
        .asset_info_from_name(asset_name, version, sha256)
        .ok()
        .flatten()
}

#[cfg(test)]
pub(super) fn legacy_asset_info_from_asset(
    asset: &GitHubAsset,
    version: &str,
) -> Result<Option<ReleaseAssetInfo>> {
    let template = AssetTemplate::new("{binary}-{version}-{target}.tar.gz".to_owned())?;
    template.asset_info_from_asset(asset, version)
}

pub(super) fn sha256_from_asset_digest(asset: &GitHubAsset) -> Result<Option<String>> {
    let Some(digest) = asset.digest.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let Some(sha256) = digest.strip_prefix("sha256:") else {
        return Err(Error::InvalidAssetDigest {
            asset_name: asset.name.clone(),
            digest: digest.to_owned(),
        });
    };
    if sha256.len() == 64 && sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(Some(sha256.to_owned()));
    }
    Err(Error::InvalidAssetDigest {
        asset_name: asset.name.clone(),
        digest: digest.to_owned(),
    })
}

fn parse_asset_name(
    template: &str,
    asset_name: &str,
    expected_version: &str,
) -> Result<Option<ParsedAsset>> {
    let parts = template_parts(template)?;
    Ok(parse_from(
        &parts,
        asset_name,
        expected_version,
        0,
        0,
        Captures::default(),
    ))
}

#[derive(Clone, Default)]
struct Captures {
    binary: Option<String>,
    target: Option<String>,
}

fn parse_from(
    parts: &[TemplatePart],
    asset_name: &str,
    expected_version: &str,
    index: usize,
    cursor: usize,
    captures: Captures,
) -> Option<ParsedAsset> {
    if index == parts.len() {
        if cursor != asset_name.len() {
            return None;
        }
        return Some(ParsedAsset {
            binary: captures.binary.filter(|value| !value.is_empty())?,
            target: captures.target.filter(|value| !value.is_empty())?,
        });
    }
    match &parts[index] {
        TemplatePart::Literal(literal) => {
            let rest = asset_name.get(cursor..)?;
            if !rest.starts_with(literal) {
                return None;
            }
            parse_from(
                parts,
                asset_name,
                expected_version,
                index + 1,
                cursor + literal.len(),
                captures,
            )
        }
        TemplatePart::Field(TemplateField::Version) => {
            let end = cursor + expected_version.len();
            let value = asset_name.get(cursor..end)?;
            if value != expected_version {
                return None;
            }
            parse_from(
                parts,
                asset_name,
                expected_version,
                index + 1,
                end,
                captures,
            )
        }
        TemplatePart::Field(field) => {
            for end in cursor + 1..=asset_name.len() {
                let Some(value) = asset_name.get(cursor..end) else {
                    continue;
                };
                let mut next = captures.clone();
                match field {
                    TemplateField::Binary => next.binary = Some(value.to_owned()),
                    TemplateField::Target => next.target = Some(value.to_owned()),
                    TemplateField::Version => return None,
                }
                if let Some(parsed) =
                    parse_from(parts, asset_name, expected_version, index + 1, end, next)
                {
                    return Some(parsed);
                }
            }
            None
        }
    }
}

fn template_parts(template: &str) -> Result<Vec<TemplatePart>> {
    let mut parts = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        if start > 0 {
            parts.push(TemplatePart::Literal(rest[..start].to_owned()));
        }
        let Some(end) = rest[start..].find('}') else {
            return Err(Error::InvalidAssetTemplate {
                template: template.to_owned(),
            });
        };
        let token = &rest[start + 1..start + end];
        let field = match token {
            "binary" => TemplateField::Binary,
            "version" => TemplateField::Version,
            "target" => TemplateField::Target,
            _ => {
                return Err(Error::InvalidAssetTemplate {
                    template: template.to_owned(),
                });
            }
        };
        if matches!(parts.last(), Some(TemplatePart::Field(_))) {
            return Err(Error::InvalidAssetTemplate {
                template: template.to_owned(),
            });
        }
        parts.push(TemplatePart::Field(field));
        rest = &rest[start + end + 1..];
    }
    if !rest.is_empty() {
        parts.push(TemplatePart::Literal(rest.to_owned()));
    }
    Ok(parts)
}

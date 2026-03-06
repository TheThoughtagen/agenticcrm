use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::blocking::Client;
use reqwest::Method;
use url::Url;

/// A single vCard resource entry from an address book listing.
pub struct VCardEntry {
    /// URL path to the vCard resource
    pub href: String,
    /// ETag for change detection
    pub etag: String,
}

/// CardDAV client for iCloud contact sync.
pub struct CardDavClient {
    client: Client,
    apple_id: String,
    app_password: String,
    base_url: Url,
}

impl CardDavClient {
    /// Create a new CardDAV client with the given iCloud credentials.
    pub fn new(apple_id: &str, app_password: &str) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("Failed to build HTTP client")?;

        let base_url =
            Url::parse("https://contacts.icloud.com/").context("Failed to parse base URL")?;

        Ok(Self {
            client,
            apple_id: apple_id.to_string(),
            app_password: app_password.to_string(),
            base_url,
        })
    }

    /// Discover the user's address book URL via the CardDAV discovery chain.
    ///
    /// Three-step PROPFIND sequence:
    /// 1. Get current-user-principal from base URL
    /// 2. Get addressbook-home-set from principal URL
    /// 3. Find addressbook collection from home URL
    pub fn discover_address_book(&self) -> Result<Url> {
        // Step 1: Discover current-user-principal
        let principal_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal/>
  </d:prop>
</d:propfind>"#;

        let response = self.propfind(&self.base_url, "0", principal_body)?;
        let principal_href = parse_principal(&response)
            .context("Failed to parse current-user-principal from response")?;
        let principal_url = self.base_url.join(&principal_href)?;

        // Step 2: Discover addressbook-home-set
        let home_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <card:addressbook-home-set/>
  </d:prop>
</d:propfind>"#;

        let response = self.propfind(&principal_url, "0", home_body)?;
        let home_href = parse_addressbook_home(&response)
            .context("Failed to parse addressbook-home-set from response")?;
        let home_url = self.base_url.join(&home_href)?;

        // Step 3: Find addressbook collection
        let collection_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cs="http://calendarserver.org/ns/">
  <d:prop>
    <d:resourcetype/>
    <d:displayname/>
    <cs:getctag/>
  </d:prop>
</d:propfind>"#;

        let response = self.propfind(&home_url, "1", collection_body)?;
        let addressbook_href = parse_addressbook_collection(&response)
            .context("Failed to find addressbook collection in response")?;
        let addressbook_url = self.base_url.join(&addressbook_href)?;

        Ok(addressbook_url)
    }

    /// Fetch the list of all vCard entries (href + etag) from an address book.
    pub fn fetch_vcard_list(&self, addressbook_url: &Url) -> Result<Vec<VCardEntry>> {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:getetag/>
    <d:resourcetype/>
  </d:prop>
</d:propfind>"#;

        let response = self.propfind(addressbook_url, "1", body)?;
        parse_vcard_entries(&response).context("Failed to parse vCard list from response")
    }

    /// Fetch the raw vCard text for a single contact by URL.
    pub fn fetch_vcard(&self, vcard_url: &Url) -> Result<String> {
        let response = self
            .client
            .get(vcard_url.as_str())
            .basic_auth(&self.apple_id, Some(&self.app_password))
            .send()
            .context("Failed to send GET request for vCard")?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 401 {
                anyhow::bail!(
                    "Authentication failed - ensure you're using an app-specific password from appleid.apple.com"
                );
            }
            anyhow::bail!("GET request failed with status {}", status);
        }

        response
            .text()
            .context("Failed to read vCard response body")
    }

    /// Send a PROPFIND request with basic auth and return the response body.
    fn propfind(&self, url: &Url, depth: &str, body: &str) -> Result<String> {
        let method = Method::from_bytes(b"PROPFIND")
            .context("Failed to create PROPFIND method")?;

        let response = self
            .client
            .request(method, url.as_str())
            .header("Depth", depth)
            .header("Content-Type", "application/xml; charset=utf-8")
            .basic_auth(&self.apple_id, Some(&self.app_password))
            .body(body.to_string())
            .send()
            .context("Failed to send PROPFIND request")?;

        let status = response.status();
        match status.as_u16() {
            207 => {} // Multi-Status - expected
            401 => {
                anyhow::bail!(
                    "Authentication failed - ensure you're using an app-specific password from appleid.apple.com"
                );
            }
            403 => {
                anyhow::bail!("Access forbidden (403). Check your iCloud account permissions.");
            }
            code => {
                anyhow::bail!("PROPFIND request failed with status {}", code);
            }
        }

        response
            .text()
            .context("Failed to read PROPFIND response body")
    }
}

/// Extract the current-user-principal href from a PROPFIND response.
fn parse_principal(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_principal = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                if local == "current-user-principal" {
                    in_principal = true;
                } else if in_principal && local == "href" {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        let href = t.xml_content()?.to_string();
                        return Ok(href);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                if local == "current-user-principal" {
                    in_principal = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("XML parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }

    anyhow::bail!("current-user-principal not found in response")
}

/// Extract the addressbook-home-set href from a PROPFIND response.
fn parse_addressbook_home(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_home_set = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                if local == "addressbook-home-set" {
                    in_home_set = true;
                } else if in_home_set && local == "href" {
                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                        let href = t.xml_content()?.to_string();
                        return Ok(href);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                if local == "addressbook-home-set" {
                    in_home_set = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("XML parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }

    anyhow::bail!("addressbook-home-set not found in response")
}

/// Find the address book collection href from a depth-1 PROPFIND response.
///
/// Looks for a response entry whose resourcetype contains an `addressbook` element.
fn parse_addressbook_collection(xml: &str) -> Result<String> {
    // Parse response entries: each <response> has an <href> and <propstat> with <resourcetype>
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    // Track state per <response> element
    let mut in_response = false;
    let mut in_propstat = false;
    let mut in_resourcetype = false;
    let mut current_href: Option<String> = None;
    let mut is_addressbook = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "response" => {
                        in_response = true;
                        current_href = None;
                        is_addressbook = false;
                    }
                    "propstat" if in_response => {
                        in_propstat = true;
                    }
                    "resourcetype" if in_propstat => {
                        in_resourcetype = true;
                    }
                    "href" if in_response && !in_propstat => {
                        // The href directly inside <response> (not inside <propstat>)
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            current_href = Some(t.xml_content()?.to_string());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                if local == "addressbook" && in_resourcetype {
                    is_addressbook = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "response" => {
                        if is_addressbook {
                            if let Some(href) = current_href {
                                return Ok(href);
                            }
                        }
                        in_response = false;
                    }
                    "propstat" => {
                        in_propstat = false;
                    }
                    "resourcetype" => {
                        in_resourcetype = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("XML parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }

    anyhow::bail!("No addressbook collection found in response")
}

/// Parse a multi-status response to extract vCard entries (non-collection resources).
///
/// Returns a list of VCardEntry with href and etag for each vCard resource.
fn parse_vcard_entries(xml: &str) -> Result<Vec<VCardEntry>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut entries = Vec::new();

    // Track state per <response> element
    let mut in_response = false;
    let mut in_propstat = false;
    let mut in_resourcetype = false;
    let mut current_href: Option<String> = None;
    let mut current_etag: Option<String> = None;
    let mut is_collection = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "response" => {
                        in_response = true;
                        current_href = None;
                        current_etag = None;
                        is_collection = false;
                    }
                    "propstat" if in_response => {
                        in_propstat = true;
                    }
                    "resourcetype" if in_propstat => {
                        in_resourcetype = true;
                    }
                    "href" if in_response && !in_propstat => {
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            current_href = Some(t.xml_content()?.to_string());
                        }
                    }
                    "getetag" if in_propstat => {
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            let etag = t.xml_content()?.to_string();
                            // Strip surrounding quotes from etag if present
                            let etag = etag.trim_matches('"').to_string();
                            current_etag = Some(etag);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                if in_resourcetype && (local == "collection" || local == "addressbook") {
                    is_collection = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "response" => {
                        if !is_collection {
                            if let (Some(href), Some(etag)) =
                                (current_href.take(), current_etag.take())
                            {
                                entries.push(VCardEntry { href, etag });
                            }
                        }
                        in_response = false;
                    }
                    "propstat" => {
                        in_propstat = false;
                    }
                    "resourcetype" => {
                        in_resourcetype = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("XML parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

/// Extract local element name from a potentially namespace-prefixed name.
/// e.g., "d:href" -> "href", "card:addressbook-home-set" -> "addressbook-home-set"
/// Returns an owned String to avoid lifetime issues with quick-xml temporaries.
fn local_name(name: &[u8]) -> String {
    let name_str = std::str::from_utf8(name).unwrap_or("");
    if let Some(pos) = name_str.rfind(':') {
        name_str[pos + 1..].to_string()
    } else {
        name_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_principal() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/</d:href>
    <d:propstat>
      <d:prop>
        <d:current-user-principal>
          <d:href>/123456789/principal/</d:href>
        </d:current-user-principal>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let result = parse_principal(xml).unwrap();
        assert_eq!(result, "/123456789/principal/");
    }

    #[test]
    fn test_parse_principal_not_found() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/</d:href>
    <d:propstat>
      <d:prop/>
      <d:status>HTTP/1.1 404 Not Found</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        assert!(parse_principal(xml).is_err());
    }

    #[test]
    fn test_parse_addressbook_home() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/123456789/principal/</d:href>
    <d:propstat>
      <d:prop>
        <card:addressbook-home-set>
          <d:href>/123456789/carddavhome/</d:href>
        </card:addressbook-home-set>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let result = parse_addressbook_home(xml).unwrap();
        assert_eq!(result, "/123456789/carddavhome/");
    }

    #[test]
    fn test_parse_addressbook_collection() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cs="http://calendarserver.org/ns/">
  <d:response>
    <d:href>/123456789/carddavhome/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype>
          <d:collection/>
        </d:resourcetype>
        <d:displayname>CardDAV Home</d:displayname>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/123456789/carddavhome/card/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype>
          <d:collection/>
          <card:addressbook/>
        </d:resourcetype>
        <d:displayname>Contacts</d:displayname>
        <cs:getctag>abcd-1234</cs:getctag>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let result = parse_addressbook_collection(xml).unwrap();
        assert_eq!(result, "/123456789/carddavhome/card/");
    }

    #[test]
    fn test_parse_addressbook_collection_not_found() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/123456789/carddavhome/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype>
          <d:collection/>
        </d:resourcetype>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        assert!(parse_addressbook_collection(xml).is_err());
    }

    #[test]
    fn test_parse_vcard_entries() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/123456789/carddavhome/card/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype>
          <d:collection/>
          <card:addressbook/>
        </d:resourcetype>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/123456789/carddavhome/card/contact1.vcf</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"etag-aaa-111"</d:getetag>
        <d:resourcetype/>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/123456789/carddavhome/card/contact2.vcf</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"etag-bbb-222"</d:getetag>
        <d:resourcetype/>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let entries = parse_vcard_entries(xml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].href,
            "/123456789/carddavhome/card/contact1.vcf"
        );
        assert_eq!(entries[0].etag, "etag-aaa-111");
        assert_eq!(
            entries[1].href,
            "/123456789/carddavhome/card/contact2.vcf"
        );
        assert_eq!(entries[1].etag, "etag-bbb-222");
    }

    #[test]
    fn test_parse_vcard_entries_empty_addressbook() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/123456789/carddavhome/card/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype>
          <d:collection/>
          <card:addressbook/>
        </d:resourcetype>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let entries = parse_vcard_entries(xml).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_local_name() {
        assert_eq!(local_name(b"d:href"), "href");
        assert_eq!(local_name(b"card:addressbook"), "addressbook");
        assert_eq!(local_name(b"href"), "href");
        assert_eq!(local_name(b"cs:getctag"), "getctag");
    }
}

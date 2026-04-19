//! Contacts CLI commands.
//!
//! Commands for managing address books, contacts, groups, and
//! vCard import/export.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Contacts operations.
#[derive(Subcommand)]
pub enum ContactsCommand {
    /// Create an address book.
    CreateBook(CreateBookArgs),

    /// List address books.
    ListBooks(ListBooksArgs),

    /// Delete an address book and all its contacts.
    DeleteBook(DeleteBookArgs),

    /// Add a contact to an address book.
    Add(AddContactArgs),

    /// Get a contact by ID (returns vCard).
    Get(GetContactArgs),

    /// Update a contact with new vCard data.
    Update(UpdateContactArgs),

    /// Delete a contact.
    Delete(DeleteContactArgs),

    /// List contacts in an address book.
    List(ListContactsArgs),

    /// Search contacts by name, email, or phone.
    Search(SearchContactsArgs),

    /// Import contacts from a vCard file.
    Import(ImportArgs),

    /// Export contacts as vCard format.
    Export(ExportArgs),

    /// Create a contact group.
    CreateGroup(CreateGroupArgs),

    /// List groups in an address book.
    ListGroups(ListGroupsArgs),

    /// Delete a group.
    DeleteGroup(DeleteGroupArgs),
}

#[derive(Args)]
pub struct CreateBookArgs {
    /// Address book name.
    pub name: String,
    /// Optional description.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct ListBooksArgs {
    /// Maximum number of books to return.
    #[arg(long = "limit")]
    pub max_results: Option<u32>,
}

#[derive(Args)]
pub struct DeleteBookArgs {
    /// Address book ID.
    pub book_id: String,
}

#[derive(Args)]
pub struct AddContactArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
    /// Display name.
    #[arg(long)]
    pub name: String,
    /// Email address.
    #[arg(long)]
    pub email: Option<String>,
    /// Phone number.
    #[arg(long)]
    pub phone: Option<String>,
    /// Organization.
    #[arg(long)]
    pub org: Option<String>,
}

#[derive(Args)]
pub struct GetContactArgs {
    /// Contact ID.
    pub contact_id: String,
}

#[derive(Args)]
pub struct UpdateContactArgs {
    /// Contact ID.
    pub contact_id: String,
    /// vCard data string.
    #[arg(long)]
    pub vcard: String,
}

#[derive(Args)]
pub struct DeleteContactArgs {
    /// Contact ID.
    pub contact_id: String,
}

#[derive(Args)]
pub struct ListContactsArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
    /// Maximum number of contacts to return.
    #[arg(long = "limit")]
    pub max_results: Option<u32>,
}

#[derive(Args)]
pub struct SearchContactsArgs {
    /// Search query (matches name, email, phone).
    pub query: String,
    /// Restrict search to a specific book.
    #[arg(long)]
    pub book: Option<String>,
    /// Maximum number of results.
    #[arg(long = "limit")]
    pub max_results: Option<u32>,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
    /// Path to vCard (.vcf) file.
    pub file: String,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
}

#[derive(Args)]
pub struct CreateGroupArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
    /// Group name.
    pub name: String,
}

#[derive(Args)]
pub struct ListGroupsArgs {
    /// Address book ID.
    #[arg(long)]
    pub book: String,
}

#[derive(Args)]
pub struct DeleteGroupArgs {
    /// Group ID.
    pub group_id: String,
}

impl ContactsCommand {
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            Self::CreateBook(a) => create_book(client, a, is_json_output).await,
            Self::ListBooks(a) => list_books(client, a, is_json_output).await,
            Self::DeleteBook(a) => delete_book(client, a, is_json_output).await,
            Self::Add(a) => add_contact(client, a, is_json_output).await,
            Self::Get(a) => get_contact(client, a, is_json_output).await,
            Self::Update(a) => update_contact(client, a, is_json_output).await,
            Self::Delete(a) => delete_contact(client, a, is_json_output).await,
            Self::List(a) => list_contacts(client, a, is_json_output).await,
            Self::Search(a) => search_contacts(client, a, is_json_output).await,
            Self::Import(a) => import_contacts(client, a, is_json_output).await,
            Self::Export(a) => export_contacts(client, a, is_json_output).await,
            Self::CreateGroup(a) => create_group(client, a, is_json_output).await,
            Self::ListGroups(a) => list_groups(client, a, is_json_output).await,
            Self::DeleteGroup(a) => delete_group(client, a, is_json_output).await,
        }
    }
}

fn print_contact_data_or_error(contact_data: Option<&str>, error_message: Option<&str>) {
    debug_assert!(contact_data.is_some() || error_message.is_some());
    debug_assert!(contact_data.is_none() || error_message.is_none());
    if let Some(contact_data) = contact_data {
        print!("{contact_data}");
        return;
    }
    if let Some(error_message) = error_message {
        eprintln!("Error: {error_message}");
    }
}

async fn create_book(client: &AspenClient, args: CreateBookArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty());
    debug_assert!(args.description.as_deref().is_none_or(|description| !description.is_empty()));
    let resp = client
        .send(ClientRpcRequest::ContactsCreateBook {
            name: args.name,
            description: args.description,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsBookResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.book_id,
                name: r.name,
                error: r.error,
                label: "book",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_books(client: &AspenClient, args: ListBooksArgs, is_json_output: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsListBooks {
            limit: args.max_results,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsBookListResult(r) => {
            let output = BookListOutput {
                books: r.books,
                error: r.error,
            };
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_book(client: &AspenClient, args: DeleteBookArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book_id.is_empty());
    debug_assert_eq!(args.book_id, args.book_id.trim());
    let resp = client.send(ClientRpcRequest::ContactsDeleteBook { book_id: args.book_id }).await?;
    match resp {
        ClientRpcResponse::ContactsBookResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.book_id,
                name: None,
                error: r.error,
                label: "book",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn add_contact(client: &AspenClient, args: AddContactArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book.is_empty());
    debug_assert!(!args.name.is_empty());
    // Build a minimal vCard from CLI args.
    let mut vcard = format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:{}\r\n", args.name);
    if let Some(email) = &args.email {
        vcard.push_str(&format!("EMAIL:{email}\r\n"));
    }
    if let Some(phone) = &args.phone {
        vcard.push_str(&format!("TEL:{phone}\r\n"));
    }
    if let Some(org) = &args.org {
        vcard.push_str(&format!("ORG:{org}\r\n"));
    }
    vcard.push_str("END:VCARD\r\n");

    let resp = client
        .send(ClientRpcRequest::ContactsCreateContact {
            book_id: args.book,
            vcard_data: vcard,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.contact_id,
                name: None,
                error: r.error,
                label: "contact",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn get_contact(client: &AspenClient, args: GetContactArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.contact_id.is_empty());
    debug_assert_eq!(args.contact_id, args.contact_id.trim());
    let resp = client
        .send(ClientRpcRequest::ContactsGetContact {
            contact_id: args.contact_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            let output = VcardOutput {
                vcard_data: r.vcard_data,
                contact_id: r.contact_id,
                error: r.error,
            };
            debug_assert!(output.vcard_data.is_some() || output.error.is_some());
            debug_assert!(output.contact_id.is_some() || output.error.is_some());
            if is_json_output {
                print_output(&output, is_json_output);
            } else {
                print_contact_data_or_error(output.vcard_data.as_deref(), output.error.as_deref());
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn update_contact(client: &AspenClient, args: UpdateContactArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.contact_id.is_empty());
    debug_assert!(!args.vcard.is_empty());
    let resp = client
        .send(ClientRpcRequest::ContactsUpdateContact {
            contact_id: args.contact_id,
            vcard_data: args.vcard,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.contact_id,
                name: None,
                error: r.error,
                label: "contact",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_contact(client: &AspenClient, args: DeleteContactArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.contact_id.is_empty());
    debug_assert_eq!(args.contact_id, args.contact_id.trim());
    let resp = client
        .send(ClientRpcRequest::ContactsDeleteContact {
            contact_id: args.contact_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.contact_id,
                name: None,
                error: r.error,
                label: "contact",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_contacts(client: &AspenClient, args: ListContactsArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book.is_empty());
    debug_assert!(args.max_results.is_none_or(|limit| limit > 0));
    let resp = client
        .send(ClientRpcRequest::ContactsListContacts {
            book_id: args.book,
            limit: args.max_results,
            continuation_token: None,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsListResult(r) => {
            let output = ContactListOutput {
                contacts: r.contacts,
                total: r.total,
                error: r.error,
            };
            debug_assert!(output.total >= u32::try_from(output.contacts.len()).unwrap_or(u32::MAX));
            debug_assert!(output.error.is_none() || output.contacts.is_empty());
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn search_contacts(client: &AspenClient, args: SearchContactsArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.query.is_empty());
    debug_assert!(args.max_results.is_none_or(|limit| limit > 0));
    let resp = client
        .send(ClientRpcRequest::ContactsSearchContacts {
            query: args.query,
            book_id: args.book,
            limit: args.max_results,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsSearchResult(r) => {
            let output = ContactListOutput {
                contacts: r.contacts,
                total: r.total,
                error: r.error,
            };
            debug_assert!(output.total >= u32::try_from(output.contacts.len()).unwrap_or(u32::MAX));
            debug_assert!(output.error.is_none() || output.contacts.is_empty());
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn import_contacts(client: &AspenClient, args: ImportArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book.is_empty());
    debug_assert!(!args.file.is_empty());
    let vcard_data =
        std::fs::read_to_string(&args.file).map_err(|e| anyhow::anyhow!("failed to read {}: {}", args.file, e))?;
    let resp = client
        .send(ClientRpcRequest::ContactsImportVcard {
            book_id: args.book,
            vcard_data,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsExportResult(r) => {
            let output = ImportExportOutput {
                count: r.count,
                is_success: r.is_success,
                error: r.error,
                op: "imported",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.count > 0 || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn export_contacts(client: &AspenClient, args: ExportArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book.is_empty());
    debug_assert_eq!(args.book, args.book.trim());
    let resp = client.send(ClientRpcRequest::ContactsExportVcard { book_id: args.book }).await?;
    match resp {
        ClientRpcResponse::ContactsExportResult(r) => {
            let output = ImportExportOutput {
                count: r.count,
                is_success: r.is_success,
                error: r.error.clone(),
                op: "exported",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(r.vcard_data.is_some() || output.error.is_some());
            if is_json_output {
                print_output(&output, is_json_output);
            } else {
                print_contact_data_or_error(r.vcard_data.as_deref(), output.error.as_deref());
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn create_group(client: &AspenClient, args: CreateGroupArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.book.is_empty());
    debug_assert!(!args.name.is_empty());
    let resp = client
        .send(ClientRpcRequest::ContactsCreateGroup {
            book_id: args.book,
            name: args.name,
            member_ids: vec![],
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.group_id,
                name: r.name,
                error: r.error,
                label: "group",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_groups(client: &AspenClient, args: ListGroupsArgs, is_json_output: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::ContactsListGroups { book_id: args.book }).await?;
    match resp {
        ClientRpcResponse::ContactsGroupListResult(r) => {
            let output = GroupListOutput {
                groups: r.groups,
                error: r.error,
            };
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_group(client: &AspenClient, args: DeleteGroupArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.group_id.is_empty());
    debug_assert_eq!(args.group_id, args.group_id.trim());
    let resp = client
        .send(ClientRpcRequest::ContactsDeleteGroup {
            group_id: args.group_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            let output = SimpleResult {
                is_success: r.is_success,
                id: r.group_id,
                name: None,
                error: r.error,
                label: "group",
            };
            debug_assert!(output.is_success || output.error.is_some());
            debug_assert!(output.id.is_some() || !output.is_success);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

// =========================================================================
// Output types
// =========================================================================

struct SimpleResult {
    is_success: bool,
    id: Option<String>,
    name: Option<String>,
    error: Option<String>,
    label: &'static str,
}

impl Outputable for SimpleResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "success": self.is_success, "id": self.id, "name": self.name, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.is_success {
            let id = self.id.as_deref().unwrap_or("?");
            match &self.name {
                Some(n) => format!("{} '{}' ({})", self.label, n, id),
                None => format!("{} {}", self.label, id),
            }
        } else {
            format!("Error: {}", self.error.as_deref().unwrap_or("unknown"))
        }
    }
}

struct VcardOutput {
    vcard_data: Option<String>,
    contact_id: Option<String>,
    error: Option<String>,
}

impl Outputable for VcardOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "contact_id": self.contact_id, "vcard": self.vcard_data, "error": self.error })
    }
    fn to_human(&self) -> String {
        self.vcard_data
            .clone()
            .unwrap_or_else(|| format!("Error: {}", self.error.as_deref().unwrap_or("not found")))
    }
}

struct BookListOutput {
    books: Vec<aspen_client_api::contacts::ContactsBookInfo>,
    error: Option<String>,
}

impl Outputable for BookListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "books": self.books.iter().map(|b| serde_json::json!({"id": b.id, "name": b.name, "description": b.description, "contact_count": b.contact_count})).collect::<Vec<_>>(), "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.books.is_empty() {
            return "No address books found.".to_string();
        }
        let mut lines = vec![format!("{:<20} {:<24} {}", "NAME", "ID", "CONTACTS")];
        for b in &self.books {
            lines.push(format!("{:<20} {:<24} {}", b.name, b.id, b.contact_count));
        }
        lines.join("\n")
    }
}

struct ContactListOutput {
    contacts: Vec<aspen_client_api::contacts::ContactSummary>,
    total: u32,
    error: Option<String>,
}

impl Outputable for ContactListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "contacts": self.contacts.iter().map(|c| serde_json::json!({"id": c.id, "name": c.display_name, "email": c.primary_email, "phone": c.primary_phone})).collect::<Vec<_>>(), "total": self.total, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.contacts.is_empty() {
            return "No contacts found.".to_string();
        }
        let mut lines = vec![format!("{:<24} {:<24} {:<20} {}", "NAME", "EMAIL", "PHONE", "ID")];
        for c in &self.contacts {
            lines.push(format!(
                "{:<24} {:<24} {:<20} {}",
                c.display_name,
                c.primary_email.as_deref().unwrap_or("-"),
                c.primary_phone.as_deref().unwrap_or("-"),
                c.id
            ));
        }
        lines.join("\n")
    }
}

struct GroupListOutput {
    groups: Vec<aspen_client_api::contacts::ContactsGroupInfo>,
    error: Option<String>,
}

impl Outputable for GroupListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "groups": self.groups.iter().map(|g| serde_json::json!({"id": g.id, "name": g.name, "member_count": g.member_count})).collect::<Vec<_>>(), "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.groups.is_empty() {
            return "No groups found.".to_string();
        }
        let mut lines = vec![format!("{:<20} {:<24} {}", "NAME", "ID", "MEMBERS")];
        for g in &self.groups {
            lines.push(format!("{:<20} {:<24} {}", g.name, g.id, g.member_count));
        }
        lines.join("\n")
    }
}

struct ImportExportOutput {
    count: u32,
    is_success: bool,
    error: Option<String>,
    op: &'static str,
}

impl Outputable for ImportExportOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "success": self.is_success, "count": self.count, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.is_success {
            format!("{} {} contacts", self.op, self.count)
        } else {
            format!("Error: {}", self.error.as_deref().unwrap_or("unknown"))
        }
    }
}

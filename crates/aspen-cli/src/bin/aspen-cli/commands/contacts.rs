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
    #[arg(long)]
    pub limit: Option<u32>,
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
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Args)]
pub struct SearchContactsArgs {
    /// Search query (matches name, email, phone).
    pub query: String,
    /// Restrict search to a specific book.
    #[arg(long)]
    pub book: Option<String>,
    /// Maximum number of results.
    #[arg(long)]
    pub limit: Option<u32>,
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
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            Self::CreateBook(a) => create_book(client, a, json).await,
            Self::ListBooks(a) => list_books(client, a, json).await,
            Self::DeleteBook(a) => delete_book(client, a, json).await,
            Self::Add(a) => add_contact(client, a, json).await,
            Self::Get(a) => get_contact(client, a, json).await,
            Self::Update(a) => update_contact(client, a, json).await,
            Self::Delete(a) => delete_contact(client, a, json).await,
            Self::List(a) => list_contacts(client, a, json).await,
            Self::Search(a) => search_contacts(client, a, json).await,
            Self::Import(a) => import_contacts(client, a, json).await,
            Self::Export(a) => export_contacts(client, a, json).await,
            Self::CreateGroup(a) => create_group(client, a, json).await,
            Self::ListGroups(a) => list_groups(client, a, json).await,
            Self::DeleteGroup(a) => delete_group(client, a, json).await,
        }
    }
}

async fn create_book(client: &AspenClient, args: CreateBookArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsCreateBook {
            name: args.name,
            description: args.description,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsBookResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.book_id,
                    name: r.name,
                    error: r.error,
                    label: "book",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_books(client: &AspenClient, args: ListBooksArgs, json: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::ContactsListBooks { limit: args.limit }).await?;
    match resp {
        ClientRpcResponse::ContactsBookListResult(r) => {
            let output = BookListOutput {
                books: r.books,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_book(client: &AspenClient, args: DeleteBookArgs, json: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::ContactsDeleteBook { book_id: args.book_id }).await?;
    match resp {
        ClientRpcResponse::ContactsBookResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.book_id,
                    name: None,
                    error: r.error,
                    label: "book",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn add_contact(client: &AspenClient, args: AddContactArgs, json: bool) -> Result<()> {
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
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.contact_id,
                    name: None,
                    error: r.error,
                    label: "contact",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn get_contact(client: &AspenClient, args: GetContactArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsGetContact {
            contact_id: args.contact_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            if json {
                print_output(
                    &VcardOutput {
                        vcard_data: r.vcard_data,
                        contact_id: r.contact_id,
                        error: r.error,
                    },
                    json,
                );
            } else if let Some(vcard) = &r.vcard_data {
                println!("{vcard}");
            } else if let Some(err) = &r.error {
                eprintln!("Error: {err}");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn update_contact(client: &AspenClient, args: UpdateContactArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsUpdateContact {
            contact_id: args.contact_id,
            vcard_data: args.vcard,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.contact_id,
                    name: None,
                    error: r.error,
                    label: "contact",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_contact(client: &AspenClient, args: DeleteContactArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsDeleteContact {
            contact_id: args.contact_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.contact_id,
                    name: None,
                    error: r.error,
                    label: "contact",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_contacts(client: &AspenClient, args: ListContactsArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsListContacts {
            book_id: args.book,
            limit: args.limit,
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
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn search_contacts(client: &AspenClient, args: SearchContactsArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsSearchContacts {
            query: args.query,
            book_id: args.book,
            limit: args.limit,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsSearchResult(r) => {
            let output = ContactListOutput {
                contacts: r.contacts,
                total: r.total,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn import_contacts(client: &AspenClient, args: ImportArgs, json: bool) -> Result<()> {
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
            print_output(
                &ImportExportOutput {
                    count: r.count,
                    is_success: r.is_success,
                    error: r.error,
                    op: "imported",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn export_contacts(client: &AspenClient, args: ExportArgs, json: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::ContactsExportVcard { book_id: args.book }).await?;
    match resp {
        ClientRpcResponse::ContactsExportResult(r) => {
            if json {
                print_output(
                    &ImportExportOutput {
                        count: r.count,
                        is_success: r.is_success,
                        error: r.error,
                        op: "exported",
                    },
                    json,
                );
            } else if let Some(data) = &r.vcard_data {
                print!("{data}");
            } else if let Some(err) = &r.error {
                eprintln!("Error: {err}");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn create_group(client: &AspenClient, args: CreateGroupArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsCreateGroup {
            book_id: args.book,
            name: args.name,
            member_ids: vec![],
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.group_id,
                    name: r.name,
                    error: r.error,
                    label: "group",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_groups(client: &AspenClient, args: ListGroupsArgs, json: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::ContactsListGroups { book_id: args.book }).await?;
    match resp {
        ClientRpcResponse::ContactsGroupListResult(r) => {
            let output = GroupListOutput {
                groups: r.groups,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_group(client: &AspenClient, args: DeleteGroupArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::ContactsDeleteGroup {
            group_id: args.group_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.group_id,
                    name: None,
                    error: r.error,
                    label: "group",
                },
                json,
            );
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

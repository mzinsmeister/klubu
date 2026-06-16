use leptos::*;
use shared::*;


#[server(name = GetContacts, prefix = "/api", endpoint = "get_contacts")]
pub async fn get_contacts() -> Result<Vec<Contact>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found in context"))?;
    
    let rows = sqlx::query!(
        "SELECT id, form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person FROM contact"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let contacts = rows.into_iter().map(|r| Contact {
        id: Some(r.id as i64),
        form_of_address: r.form_of_address,
        title: r.title,
        name: r.name,
        first_name: r.first_name,
        street: r.street,
        zip_code: r.zip_code,
        city: r.city,
        house_number: r.house_number,
        country: r.country,
        phone: r.phone,
        is_person: r.is_person != 0,
    }).collect();
    
    Ok(contacts)
}

#[server(name = SaveContact, prefix = "/api", endpoint = "save_contact")]
pub async fn save_contact(contact: Contact) -> Result<Contact, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    
    if let Some(id) = contact.id {
        let id_i32 = id as i32;
        let is_person_val = if contact.is_person { 1 } else { 0 };
        sqlx::query!(
            "UPDATE contact SET form_of_address = $1, title = $2, name = $3, first_name = $4, street = $5, zip_code = $6, city = $7, house_number = $8, country = $9, phone = $10, is_person = $11 WHERE id = $12",
            contact.form_of_address,
            contact.title,
            contact.name,
            contact.first_name,
            contact.street,
            contact.zip_code,
            contact.city,
            contact.house_number,
            contact.country,
            contact.phone,
            is_person_val,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(contact)
    } else {
        let is_person_val = if contact.is_person { 1 } else { 0 };
        let row = sqlx::query!(
            "INSERT INTO contact (form_of_address, title, name, first_name, street, zip_code, city, house_number, country, phone, is_person) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
            contact.form_of_address,
            contact.title,
            contact.name,
            contact.first_name,
            contact.street,
            contact.zip_code,
            contact.city,
            contact.house_number,
            contact.country,
            contact.phone,
            is_person_val
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        let mut new_contact = contact;
        new_contact.id = Some(row.id as i64);
        Ok(new_contact)
    }
}

#[server(name = DeleteContact, prefix = "/api", endpoint = "delete_contact")]
pub async fn delete_contact(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM contact WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}


import { type ApiPage } from "@/models/ApiModel";
import { type Contact } from "@/models/ContactModel";
import axios from "axios";

export async function listContacts(
  page: number,
  pageSize: number,
  name?: string
): Promise<Array<Contact>> {
  const response = await axios.get<ApiPage<Contact>>("/api/contacts", {
    params: {
      page: page,
      size: pageSize,
      name: name,
    },
  });
  return response.data.content;
}

export async function createContact(contact: Contact): Promise<Contact> {
  const response = await axios.post("/api/contacts", contact);
  return response.data;
}

export async function updateContact(contact: Contact): Promise<void> {
  if (contact.id === undefined) {
    throw Error("trying to update new contact" + JSON.stringify(contact));
  }
  await axios.put("/api/contacts/" + contact.id, contact);
}

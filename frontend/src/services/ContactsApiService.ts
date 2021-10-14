import { ApiPage } from "@/models/ApiModel";
import { Contact } from "@/models/ContactModel";
import Vue from "vue";

export async function listContacts(
  page: number,
  pageSize: number,
  name?: string
): Promise<Array<Contact>> {
  const response = await Vue.axios.get<ApiPage<Contact>>("/api/contacts", {
    params: {
      page: page,
      size: pageSize,
      name: name,
    },
  });
  return response.data.content;
}

export async function createContact(contact: Contact): Promise<Contact> {
  const response = await Vue.axios.post("/api/contacts", contact);
  return response.data;
}

export async function updateContact(contact: Contact): Promise<void> {
  if (contact.id === undefined) {
    throw Error("trying to update new contact" + JSON.stringify(contact));
  }
  await Vue.axios.put("/api/contacts/" + contact.id, contact);
}

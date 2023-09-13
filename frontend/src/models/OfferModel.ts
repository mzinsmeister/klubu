import type { Item, Recipient } from "./CommonModel";
import type { Contact } from "./ContactModel";
import type { Document } from "./DocumentModel";

export interface Offer {
  id?: number;
  revision?: number;
  title?: string;
  customerContact?: Contact;
  offerDate?: Date;
  validUntilDate?: Date;
  recipient?: Recipient;
  items: Array<Item>;
  createdTimestamp?: Date;
  committedTimestamp?: Date;
  subject?: string;
  headerHTML?: string;
  footerHTML?: string;
  document?: Document;
}

export interface OfferListItem {
  id: number;
  revision: number;
  title?: string;
  createdTimestamp: Date;
  customerContact?: Contact;
}

export interface OfferRevision {
  revisionNumber: number;
  creationDate: Date;
}

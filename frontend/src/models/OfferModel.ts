import { Item, Recipient } from "./CommonModel";
import { Contact } from "./ContactModel";

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
  subject?: string;
  headerHTML?: string;
  footerHTML?: string;
}

export interface OfferListItem {
  id: number;
  revision: number;
  title?: string;
  createdTimestamp: Date;
  customerContact?: Contact;
}

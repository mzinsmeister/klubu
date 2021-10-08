import { Contact } from "./ContactModel";
import { Recipent } from "./CommonModel";
import { OfferItem } from "./OfferModel";

export interface ApiPage<T> {
  content: Array<T>;
}

export interface RequestOfferDTO {
  title?: string;
  customerContactId?: number;
  items: Array<OfferItem>;
  recipent?: Recipent;
  headerHTML?: string;
  footerHTML?: string;
}

export interface ResponseOfferDTO {
  id: number;
  revision: number;
  title?: string;
  customerContact?: Contact;
  recipent?: Recipent;
  items: Array<OfferItem>;
  createdTimestamp: string;
  headerHTML?: string;
  footerHTML?: string;
}

export interface OfferListItemDTO {
  id: number;
  revision: number;
  title?: string;
  createdTimestamp: string;
  customerContact: Contact;
}

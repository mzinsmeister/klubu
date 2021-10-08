import { Money, Recipent } from "./CommonModel";
import { Contact } from "./ContactModel";

export interface Offer {
  id?: number;
  revision?: number;
  title?: string;
  customerContact?: Contact;
  recipent?: Recipent;
  items: Array<OfferItem>;
  createdTimestamp?: Date;
  headerHTML?: string;
  footerHTML?: string;
}

export interface OfferItem {
  item: string;
  quantity: number;
  unit: string;
  price: Money;
}

export interface OfferListItem {
  id: number;
  revision: number;
  title?: string;
  createdTimestamp: Date;
  customerContact?: Contact;
}

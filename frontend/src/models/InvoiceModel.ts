import { Item, Recipient } from "./CommonModel";
import { Contact } from "./ContactModel";

export interface Invoice {
  id?: number;
  items: Array<Item>;
  createdTimestamp?: Date;
  codifiedTimestamp?: Date;
  invoiceNumber?: number;
  paidDate?: Date;
  invoiceDate?: Date;
  isCanceled: boolean;
  isCancelation: boolean;
  correctedInvoiceId?: number;
  customerContact?: Contact;
  document?: Document;
  recipient?: Recipient;
  headerHTML?: string;
  footerHTML?: string;
  title?: string;
  subject?: string;
}

export interface InvoiceListItem {
  id: number;
  createdTimestamp: Date;
  customerContact?: Contact;
  paidDate?: Date;
  codified: boolean;
  invoiceNumber?: number;
  isCanceled: boolean;
  isCancelation: boolean;
}

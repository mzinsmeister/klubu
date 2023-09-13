import type { Item, Payment, Recipient } from "./CommonModel";
import type { Contact } from "./ContactModel";
import type { Document } from "./DocumentModel";

export interface Invoice {
  id?: number;
  items: Array<Item>;
  createdTimestamp?: Date;
  committedTimestamp?: Date;
  invoiceNumber?: number;
  payments: Array<Payment>;
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
  committed: boolean;
  invoiceNumber?: number;
  isCanceled: boolean;
  isCancelation: boolean;
}

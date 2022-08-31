import { Money } from "./CommonModel";
import { Contact } from "./ContactModel";
import { Document, DocumentData } from "./DocumentModel";

export interface Receipt {
  id?: number;
  items: Array<ReceiptItem>;
  createdTimestamp?: Date;
  committedTimestamp?: Date;
  receiptNumber: string;
  paidDate?: Date;
  receiptDate?: Date;
  dueDate?: Date;
  supplierContact?: Contact;
  document?: Document;
  documentData: DocumentData | null;
}

export interface ReceiptListItem {
  id: number;
  createdTimestamp: Date;
  supplierContact?: Contact;
  paidDate?: Date;
  dueDate?: Date;
  receiptDate?: Date;
  receiptNumber?: string;
}

export interface ReceiptItem {
  item: string;
  price: Money;
  category?: ReceiptItemCategory
}

export interface ReceiptItemCategory {
  id: number,
  name: string,
  categoryType: ReceiptItemCategoryType
}

export interface ReceiptItemCategoryType {
  id: number,
  name: string
}

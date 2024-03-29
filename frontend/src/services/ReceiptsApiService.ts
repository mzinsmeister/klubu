import type {
  ApiPage,
  ReceiptCommittedDTO,
  ReceiptListItemDTO,
  RequestReceiptDocumentDTO,
  RequestReceiptDTO,
  RequestReceiptItemDTO,
  ResponseReceiptDTO,
} from "@/models/ApiModel";
import { type DocumentData } from "@/models/DocumentModel";
import type { Receipt, ReceiptItem, ReceiptItemCategory, ReceiptListItem } from "@/models/ReceiptModel";
import { formatISO, parseISO } from "date-fns";
import { fromUint8Array } from "js-base64";
import axios from "axios";

export async function listReceipts(
  page: number,
  pageSize: number
): Promise<Array<ReceiptListItem>> {
  const response = await axios.get<ApiPage<ReceiptListItemDTO>>(
    "/api/receipts",
    {
      params: {
        page: page,
        size: pageSize,
      },
    }
  );
  return response.data.content.map((dto) => ({
    id: dto.id,
    createdTimestamp: parseISO(dto.createdTimestamp),
    supplierContact: dto.supplierContact,
    committed: dto.committed,
    receiptNumber: dto.receiptNumber,
    dueDate: dto.dueDate ? parseISO(dto.dueDate) : undefined,
    receiptDate: dto.receiptDate ? parseISO(dto.receiptDate) : undefined,
  }));
}

function mapReceiptDTOToReceipt(dto: ResponseReceiptDTO): Receipt {
  return {
    id: dto.id,
    supplierContact: dto.supplierContact,
    items: dto.items,
    createdTimestamp: parseISO(dto.createdTimestamp),
    committedTimestamp: dto.committedTimestamp
      ? parseISO(dto.committedTimestamp)
      : undefined,
    receiptDate: dto.receiptDate ? parseISO(dto.receiptDate) : undefined,
    dueDate: dto.dueDate ? parseISO(dto.dueDate) : undefined,
    payments: dto.payments.map((payment) => ({
      amountCents: payment.amountCents,
      date: parseISO(payment.date),
    })),
    document: dto.document,
    receiptNumber: dto.receiptNumber ?? "",
    documentData: null,
  };
}

export async function fetchReceipt(id: number): Promise<Receipt> {
  const response = await axios.get<ResponseReceiptDTO>(
    "/api/receipts/" + id
  );
  return mapReceiptDTOToReceipt(response.data);
}

function mapDocumentDataToDTO(
  data: DocumentData | undefined
): RequestReceiptDocumentDTO | undefined {
  if (data === undefined) {
    return undefined;
  } else {
    return {
      data: fromUint8Array(data.data),
      mediaType: data.mediaType,
    };
  }
}

function mapReceiptItemToTO(
  receiptItem: ReceiptItem
): RequestReceiptItemDTO {
  return {
    item: receiptItem.item,
    price: receiptItem.price,
    categoryId: receiptItem.category!.id
  }
}

function mapReceiptToDTO(
  receipt: Receipt,
  addData: boolean
): RequestReceiptDTO {
  const val = {
    supplierContactId: receipt.supplierContact?.id,
    items: receipt.items.map(it => mapReceiptItemToTO(it)),
    receiptDate: receipt.receiptDate
      ? formatISO(receipt.receiptDate, { representation: "date" })
      : undefined,
    dueDate: receipt.dueDate
      ? formatISO(receipt.dueDate, { representation: "date" })
      : undefined,
    payments: receipt.payments.map((payment) => ({
      amountCents: payment.amountCents,
      date: formatISO(payment.date, { representation: "date" }),
    })),
    receiptNumber: receipt.receiptNumber,
    documentData: addData
      ? mapDocumentDataToDTO(receipt.documentData ?? undefined)
      : undefined,
  };
  return val;
}

export async function createReceipt(receipt: Receipt): Promise<Receipt> {
  const response = await axios.post(
    "/api/receipts",
    mapReceiptToDTO(receipt, true)
  );
  return mapReceiptDTOToReceipt(response.data);
}

export async function updateReceipt(
  receipt: Receipt,
  updateDocument: boolean
): Promise<void> {
  await axios.put(
    `/api/receipts/${receipt.id}?updateDocument=${updateDocument}`,
    mapReceiptToDTO(receipt, updateDocument)
  );
}

export async function commitReceipt(
  receiptId: number
): Promise<ReceiptCommittedDTO> {
  const response = await axios.post(`/api/receipts/${receiptId}/committed`);
  return response.data;
}

export async function fetchReceiptItemCategories(): Promise<Array<ReceiptItemCategory>>  {
  const response = await axios.get("/api/receipts/itemcategories")
  return response.data;
}

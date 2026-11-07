/**
 * The page and the page size of a list. The size is shared by every list and
 * remembered on this computer; the page starts over with each visit.
 */
import { useState } from "preact/hooks";
import { PAGE_SIZE, pageOf, readPageSize } from "./browse.ts";

const STORED_SIZE = "rpp.pageSize";

function storedSize(): number {
  try {
    return readPageSize(localStorage.getItem(STORED_SIZE));
  } catch {
    return PAGE_SIZE; // Storage can be blocked; the default still works.
  }
}

function storeSize(size: number) {
  try {
    localStorage.setItem(STORED_SIZE, String(size));
  } catch {
    // Without storage the choice lasts until the page closes.
  }
}

export interface Paging {
  page: number;
  size: number;
  setPage: (page: number) => void;
  /** Changes the size and moves to the page that holds the first item shown. */
  setSize: (size: number) => void;
}

export function usePaging(): Paging {
  const [page, setPage] = useState(0);
  const [size, setCurrentSize] = useState(storedSize);
  const setSize = (next: number) => {
    setPage(pageOf(page * size, next));
    setCurrentSize(next);
    storeSize(next);
  };
  return { page, size, setPage, setSize };
}

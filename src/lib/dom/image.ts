const MAX_FILE_BYTES = 1024 * 1024;
const ACCEPTED_TYPES = ['image/png', 'image/jpeg', 'image/webp'];
/** The core takes images up to this size; a 128px PNG stays far below it. */
const MAX_RESULT_CHARS = 340_000;

/** For the file input's `accept`. */
export const ICON_IMAGE_TYPES = ACCEPTED_TYPES.join(',');

export type ShrunkImage = { dataUrl: string } | { error: string };

/** A chosen image as a PNG `data:` URL whose longer edge is at most `side` pixels. */
export async function shrinkImage(file: File, side: number): Promise<ShrunkImage> {
  if (!ACCEPTED_TYPES.includes(file.type)) {
    return { error: 'Choose a PNG, JPG or WebP image' };
  }
  if (file.size > MAX_FILE_BYTES) {
    return { error: 'Choose an image up to 1 MB' };
  }
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    return { error: 'This image could not be read. Try another file' };
  }
  const scale = Math.min(1, side / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(bitmap.width * scale));
  canvas.height = Math.max(1, Math.round(bitmap.height * scale));
  canvas.getContext('2d')?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const dataUrl = canvas.toDataURL('image/png');
  if (dataUrl.length > MAX_RESULT_CHARS) {
    return { error: 'This image is too detailed. Try a simpler one' };
  }
  return { dataUrl };
}

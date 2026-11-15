import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileArchive, FolderOpen, X } from "lucide-react";
import { CompressOptions } from "../types/vfs";

interface ArchiveModalProps {
  defaultArchiveName: string;
  destinationDir: string;
  sources: string[];
  onClose: () => void;
  onConfirmCompress: (opts: CompressOptions) => Promise<void>;
}

const FORMAT_METHODS: Record<string, string[]> = {
  "7z": ["LZMA2", "LZMA", "PPMd", "BZip2"],
  zip: ["Deflate", "Deflate64", "BZip2", "LZMA", "Copy"],
  tar: ["Store"],
  wim: ["Store"],
  xz: ["LZMA2"],
  gzip: ["Deflate"],
  bzip2: ["BZip2"],
};

const DICT_SIZES = [
  { label: "64 KB", value: "64k", mb: 0.0625 },
  { label: "1 MB", value: "1m", mb: 1 },
  { label: "2 MB", value: "2m", mb: 2 },
  { label: "4 MB", value: "4m", mb: 4 },
  { label: "8 MB", value: "8m", mb: 8 },
  { label: "16 MB", value: "16m", mb: 16 },
  { label: "32 MB", value: "32m", mb: 32 },
  { label: "64 MB", value: "64m", mb: 64 },
  { label: "128 MB", value: "128m", mb: 128 },
  { label: "256 MB", value: "256m", mb: 256 },
  { label: "512 MB", value: "512m", mb: 512 },
  { label: "1024 MB", value: "1024m", mb: 1024 },
];

const WORD_SIZES = [8, 16, 24, 32, 48, 64, 96, 128, 192, 256, 273];

const SOLID_BLOCKS = [
  { label: "Non solido", value: "off" },
  { label: "16 MB", value: "16m" },
  { label: "64 MB", value: "64m" },
  { label: "256 MB", value: "256m" },
  { label: "1 GB", value: "1g" },
  { label: "2 GB", value: "2g" },
  { label: "4 GB", value: "4g" },
  { label: "Solido", value: "on" },
];

export const ArchiveModal: React.FC<ArchiveModalProps> = ({
  defaultArchiveName,
  destinationDir,
  sources,
  onClose,
  onConfirmCompress,
}) => {
  const [targetFolder, setTargetFolder] = useState(destinationDir);
  const [baseName, setBaseName] = useState(defaultArchiveName);
  const [format, setFormat] = useState<string>("7z");
  const [compressionLevel, setCompressionLevel] = useState<number>(5);
  const [method, setMethod] = useState<string>("LZMA2");
  const [dictionarySize, setDictionarySize] = useState<string>("16m");
  const [wordSize, setWordSize] = useState<number>(32);
  const [solidBlockSize, setSolidBlockSize] = useState<string>("2g");
  const [maxThreads, setMaxThreads] = useState<number>(8);
  const [numThreads, setNumThreads] = useState<number>(4);
  const [splitVolumeSize, setSplitVolumeSize] = useState<string>("");
  const [extraParameters, setExtraParameters] = useState<string>("");

  const [updateMode, setUpdateMode] = useState<string>("add_replace");
  const [pathMode, setPathMode] = useState<string>("relative");
  const [createSfx, setCreateSfx] = useState<boolean>(false);
  const [compressSharedFiles, setCompressSharedFiles] = useState<boolean>(false);
  const [deleteAfterCompression, setDeleteAfterCompression] =
    useState<boolean>(false);

  const [password, setPassword] = useState<string>("");
  const [passwordRepeat, setPasswordRepeat] = useState<string>("");
  const [showPassword, setShowPassword] = useState<boolean>(false);
  const [encryptionMethod, setEncryptionMethod] = useState<string>("AES256");
  const [encryptFileNames, setEncryptFileNames] = useState<boolean>(false);

  useEffect(() => {
    invoke<number>("get_cpu_threads")
      .then((threads) => {
        const t = Math.max(1, threads);
        setMaxThreads(t);
        setNumThreads(t);
      })
      .catch(() => {});
  }, []);

  // Sincronizza metodo e parametri consigliati al cambio formato/livello
  const handleFormatChange = (newFmt: string) => {
    setFormat(newFmt);
    const methods = FORMAT_METHODS[newFmt] || ["LZMA2"];
    setMethod(methods[0]);
    if (newFmt !== "7z") {
      setCreateSfx(false);
      setEncryptFileNames(false);
    }
  };

  const handleLevelChange = (lvl: number) => {
    setCompressionLevel(lvl);
    if (lvl === 0) {
      setDictionarySize("64k");
    } else if (lvl === 1) {
      setDictionarySize("1m");
      setWordSize(32);
    } else if (lvl === 3) {
      setDictionarySize("4m");
      setWordSize(32);
    } else if (lvl === 5) {
      setDictionarySize("16m");
      setWordSize(32);
    } else if (lvl === 7) {
      setDictionarySize("32m");
      setWordSize(64);
    } else if (lvl === 9) {
      setDictionarySize("64m");
      setWordSize(64);
    }
  };

  // Calcolo dinamico stima RAM per Compressione e Decompressione (stile 7-Zip)
  const memoryUsage = useMemo(() => {
    if (compressionLevel === 0 || format === "tar" || format === "wim") {
      return { compMb: 2, decompMb: 2 };
    }
    const dictObj =
      DICT_SIZES.find((d) => d.value === dictionarySize) || DICT_SIZES[5];
    const dictMb = dictObj.mb;
    const threadFactor = Math.max(1, Math.ceil(numThreads / 2));
    const compMb = Math.round(dictMb * 11.5 * threadFactor + 14);
    const decompMb = Math.max(2, Math.round(dictMb + 2));
    return { compMb, decompMb };
  }, [compressionLevel, format, dictionarySize, numThreads]);

  const extSuffix = useMemo(() => {
    if (createSfx && format === "7z") return ".exe";
    if (format === "gzip") return ".gz";
    if (format === "bzip2") return ".bz2";
    return `.${format}`;
  }, [format, createSfx]);

  const fullArchivePreview = useMemo(() => {
    const clean = baseName
      .trim()
      .replace(/\.(7z|zip|tar|wim|xz|gz|bz2|exe)$/i, "");
    const sep = targetFolder.endsWith("\\") ? "" : "\\";
    return `${targetFolder}${sep}${clean || "Archivio"}${extSuffix}`;
  }, [targetFolder, baseName, extSuffix]);

  const handleBrowseFolder = async () => {
    try {
      const picked = await invoke<string | null>("pick_folder_native", {
        initialDir: targetFolder,
      });
      if (picked) {
        setTargetFolder(picked);
      }
    } catch (err) {
      alert(String(err));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password && !showPassword && password !== passwordRepeat) {
      alert("Le password inserite non coincidono.");
      return;
    }

    const opts: CompressOptions = {
      sources,
      destinationArchive: fullArchivePreview,
      format,
      compressionLevel,
      method,
      dictionarySize,
      wordSize,
      solidBlockSize,
      numThreads,
      splitVolumeSize: splitVolumeSize.trim() ? splitVolumeSize.trim() : null,
      extraParameters: extraParameters.trim() ? extraParameters.trim() : null,
      updateMode,
      pathMode,
      createSfx,
      compressSharedFiles,
      deleteAfterCompression,
      password: password ? password : null,
      encryptionMethod,
      encryptFileNames,
    };

    onClose();
    await onConfirmCompress(opts);
  };

  const availableMethods = FORMAT_METHODS[format] || ["LZMA2"];
  const isStoreOrTar =
    compressionLevel === 0 || format === "tar" || format === "wim";

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal-window sevenzip-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FileArchive size={16} color="var(--accent-color)" />
            <span>
              Aggiungi all'archivio ({sources.length} elemento/i selezionato/i)
            </span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form className="modal-body sevenzip-body" onSubmit={handleSubmit}>
          {/* RIGA SUPERIORE: PERCORSO ARCHIVIO + PULSANTE SFOGLIA "..." */}
          <div className="sz-top-archive-row">
            <label>Archivio:</label>
            <div className="sz-archive-input-group">
              <input
                type="text"
                value={baseName}
                onChange={(e) => setBaseName(e.target.value)}
                placeholder="Nome archivio"
                autoFocus
              />
              <span className="sz-preview-badge" title={fullArchivePreview}>
                {fullArchivePreview}
              </span>
              <button
                type="button"
                className="secondary-btn sz-browse-btn"
                onClick={handleBrowseFolder}
                title="Sfoglia cartella di destinazione..."
              >
                <FolderOpen size={14} />
                <span>...</span>
              </button>
            </div>
          </div>

          {/* LAYOUT A DUE COLONNE STILE 7-ZIP */}
          <div className="sz-two-columns">
            {/* COLONNA SINISTRA: PARAMETRI COMPRESSIONE */}
            <div className="sz-column">
              <div className="sz-field-row">
                <label>Formato archivio:</label>
                <select
                  value={format}
                  onChange={(e) => handleFormatChange(e.target.value)}
                >
                  <option value="7z">7z</option>
                  <option value="zip">zip</option>
                  <option value="tar">tar</option>
                  <option value="wim">wim</option>
                  <option value="xz">xz</option>
                  <option value="gzip">gzip</option>
                  <option value="bzip2">bzip2</option>
                </select>
              </div>

              <div className="sz-field-row">
                <label>Livello di compressione:</label>
                <select
                  value={compressionLevel}
                  onChange={(e) => handleLevelChange(Number(e.target.value))}
                  disabled={format === "tar" || format === "wim"}
                >
                  <option value={0}>0 - Nessuna (Store)</option>
                  <option value={1}>1 - Velocissima (Fastest)</option>
                  <option value={3}>3 - Veloce (Fast)</option>
                  <option value={5}>5 - Normale (Normal)</option>
                  <option value={7}>7 - Massima (Maximum)</option>
                  <option value={9}>9 - Ultra</option>
                </select>
              </div>

              <div className="sz-field-row">
                <label>Metodo di compressione:</label>
                <select
                  value={method}
                  onChange={(e) => setMethod(e.target.value)}
                  disabled={isStoreOrTar}
                >
                  {availableMethods.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              </div>

              <div className="sz-field-row">
                <label>Dimensione dizionario:</label>
                <select
                  value={dictionarySize}
                  onChange={(e) => setDictionarySize(e.target.value)}
                  disabled={isStoreOrTar}
                >
                  {DICT_SIZES.map((d) => (
                    <option key={d.value} value={d.value}>
                      {d.label}
                    </option>
                  ))}
                </select>
              </div>

              <div className="sz-field-row">
                <label>Dimensione parola:</label>
                <select
                  value={wordSize}
                  onChange={(e) => setWordSize(Number(e.target.value))}
                  disabled={isStoreOrTar}
                >
                  {WORD_SIZES.map((w) => (
                    <option key={w} value={w}>
                      {w}
                    </option>
                  ))}
                </select>
              </div>

              <div className="sz-field-row">
                <label>Dimensione blocco solido:</label>
                <select
                  value={solidBlockSize}
                  onChange={(e) => setSolidBlockSize(e.target.value)}
                  disabled={format !== "7z" || compressionLevel === 0}
                >
                  {SOLID_BLOCKS.map((b) => (
                    <option key={b.value} value={b.value}>
                      {b.label}
                    </option>
                  ))}
                </select>
              </div>

              <div className="sz-field-row">
                <label>Numero di thread della CPU:</label>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <select
                    value={numThreads}
                    onChange={(e) => setNumThreads(Number(e.target.value))}
                    style={{ flex: 1 }}
                  >
                    {Array.from({ length: maxThreads }, (_, i) => i + 1).map(
                      (n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      )
                    )}
                  </select>
                  <span className="sz-threads-max">/ {maxThreads}</span>
                </div>
              </div>

              {/* Box Stima Memoria RAM stile 7-Zip */}
              <div className="sz-memory-box">
                <div className="sz-mem-line">
                  <span>Utilizzo memoria per Compressione:</span>
                  <strong>{memoryUsage.compMb} MB</strong>
                </div>
                <div className="sz-mem-line">
                  <span>Utilizzo memoria per Decompressione:</span>
                  <strong>{memoryUsage.decompMb} MB</strong>
                </div>
              </div>

              <div className="sz-field-row">
                <label>Dividi in volumi, byte:</label>
                <input
                  type="text"
                  list="kzip-volume-presets"
                  value={splitVolumeSize}
                  onChange={(e) => setSplitVolumeSize(e.target.value)}
                  placeholder="Non dividere (es. 100M, 700M, 4480M)"
                />
                <datalist id="kzip-volume-presets">
                  <option value="10M">10 MB</option>
                  <option value="100M">100 MB</option>
                  <option value="700M">700 MB (CD)</option>
                  <option value="4480M">4480 MB (DVD)</option>
                </datalist>
              </div>

              <div className="sz-field-row">
                <label>Parametri:</label>
                <input
                  type="text"
                  value={extraParameters}
                  onChange={(e) => setExtraParameters(e.target.value)}
                  placeholder="Opzionali (es. x=9)"
                />
              </div>
            </div>

            {/* COLONNA DESTRA: MODALITÀ, OPZIONI E CRITTOGRAFIA */}
            <div className="sz-column">
              <div className="sz-field-row">
                <label>Modalità di aggiornamento:</label>
                <select
                  value={updateMode}
                  onChange={(e) => setUpdateMode(e.target.value)}
                >
                  <option value="add_replace">
                    Aggiungi e sostituisci file
                  </option>
                  <option value="update_add">Aggiorna e aggiungi file</option>
                  <option value="update_existing">
                    Aggiorna solo i file esistenti
                  </option>
                  <option value="sync">Sincronizza i file</option>
                </select>
              </div>

              <div className="sz-field-row">
                <label>Modalità percorso:</label>
                <select
                  value={pathMode}
                  onChange={(e) => setPathMode(e.target.value)}
                >
                  <option value="relative">Percorsi relativi</option>
                  <option value="full">Percorsi completi</option>
                  <option value="absolute">Percorsi assoluti</option>
                </select>
              </div>

              <fieldset className="sz-groupbox">
                <legend>Opzioni</legend>
                <label className="sz-checkbox">
                  <input
                    type="checkbox"
                    checked={createSfx}
                    disabled={format !== "7z"}
                    onChange={(e) => setCreateSfx(e.target.checked)}
                  />
                  <span>Crea archivio autoestraente (SFX)</span>
                </label>
                <label className="sz-checkbox">
                  <input
                    type="checkbox"
                    checked={compressSharedFiles}
                    onChange={(e) => setCompressSharedFiles(e.target.checked)}
                  />
                  <span>Comprimi file condivisi</span>
                </label>
                <label className="sz-checkbox">
                  <input
                    type="checkbox"
                    checked={deleteAfterCompression}
                    onChange={(e) =>
                      setDeleteAfterCompression(e.target.checked)
                    }
                  />
                  <span>Elimina file dopo la compressione</span>
                </label>
              </fieldset>

              <fieldset className="sz-groupbox">
                <legend>Crittografia</legend>
                <div className="sz-field-col">
                  <label>Inserisci password:</label>
                  <input
                    type={showPassword ? "text" : "password"}
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="Nessuna password"
                  />
                </div>

                <div className="sz-field-col">
                  <label>Reinserisci password:</label>
                  <input
                    type={showPassword ? "text" : "password"}
                    value={passwordRepeat}
                    disabled={showPassword}
                    onChange={(e) => setPasswordRepeat(e.target.value)}
                    placeholder="Conferma password"
                  />
                </div>

                <label className="sz-checkbox">
                  <input
                    type="checkbox"
                    checked={showPassword}
                    onChange={(e) => setShowPassword(e.target.checked)}
                  />
                  <span>Mostra password</span>
                </label>

                <div className="sz-field-row" style={{ marginTop: 4 }}>
                  <label>Metodo di crittografia:</label>
                  <select
                    value={encryptionMethod}
                    onChange={(e) => setEncryptionMethod(e.target.value)}
                    disabled={format !== "7z" && format !== "zip"}
                  >
                    <option value="AES256">AES-256</option>
                    {format === "zip" && (
                      <option value="ZipCrypto">ZipCrypto</option>
                    )}
                  </select>
                </div>

                <label className="sz-checkbox">
                  <input
                    type="checkbox"
                    checked={encryptFileNames}
                    disabled={format !== "7z"}
                    onChange={(e) => setEncryptFileNames(e.target.checked)}
                  />
                  <span>Crittografa i nomi dei file</span>
                </label>
              </fieldset>
            </div>
          </div>

          <div className="sz-footer-buttons">
            <button type="submit" className="primary-btn">
              OK — Avvia Compressione
            </button>
            <button type="button" className="secondary-btn" onClick={onClose}>
              Annulla
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

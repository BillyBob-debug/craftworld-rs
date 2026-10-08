// Authored bounded external experiment for U1. Provenance: docs/u1-translation.md.
// Compares the Rust translation_dump JSON with Toolkit RTranslationTable at the pin:
// for every distinct key, Toolkit translate(key) must equal the Rust last-row text,
// and makeLamsKeyID(tag) must equal the Rust key for legacy text rows.
import com.google.gson.*;
import cwlib.resources.RTranslationTable;
import java.nio.file.*;
import java.util.*;

public class TranslationOracle {
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("FILE.trans RUST-DUMP.json");
        byte[] data = Files.readAllBytes(Path.of(args[0]));
        JsonObject result = JsonParser.parseString(Files.readString(Path.of(args[1])))
            .getAsJsonObject().getAsJsonObject("result");
        if (!result.get("status").getAsString().equals("parsed")) {
            try { new RTranslationTable(data); System.out.println("MISMATCH rust rejected, Toolkit parsed"); System.exit(1); }
            catch (Exception expected) { System.out.println("both rejected: " + expected.getClass().getSimpleName()); return; }
        }
        RTranslationTable table = new RTranslationTable(data);
        Map<Long, String> last = new LinkedHashMap<>();
        int tagMismatch = 0;
        for (JsonElement e : result.getAsJsonArray("rows")) {
            JsonObject row = e.getAsJsonObject();
            long key = Long.parseLong(row.get("key").getAsString(), 16);
            last.put(key, row.get("text").isJsonNull() ? null : row.get("text").getAsString());
            if (!row.get("tag").isJsonNull() && RTranslationTable.makeLamsKeyID(row.get("tag").getAsString()) != key) tagMismatch++;
        }
        int match = 0, mismatch = 0;
        for (Map.Entry<Long, String> entry : last.entrySet()) {
            if (Objects.equals(table.translate(entry.getKey()), entry.getValue())) match++;
            else { if (mismatch < 5) System.out.printf("MISMATCH key %08x%n", entry.getKey()); mismatch++; }
        }
        System.out.printf("Translation oracle: %d distinct keys, %d equal, %d differ, %d tag hash differences%n",
            last.size(), match, mismatch, tagMismatch);
        if (mismatch != 0 || tagMismatch != 0) System.exit(1);
    }
}

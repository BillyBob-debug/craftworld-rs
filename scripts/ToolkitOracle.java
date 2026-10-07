// Authored, metadata-only harness around unmodified pinned Toolkit public readers.
// Toolkit MIT pin d6a2ef80b5a09999f08f23a803b516096055e5c9; see notices.
// No save/import/decrypt calls. Resource constructor allowed only on bounded clear
// binary samples; its plain decompressor runs, but no resource payload is exported.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.types.archives.*;
import cwlib.types.databases.*;
import cwlib.types.data.*;
import java.io.*;
import java.nio.file.*;
import java.util.*;

public class ToolkitOracle {
    static String guid(GUID g) { return g == null ? null : String.format("%08x", g.getValue()); }
    static String hash(SHA1 h) { return h == null ? null : h.toString(); }
    static Map<String,Object> row(Object... fields) {
        Map<String,Object> out = new TreeMap<>();
        for (int i=0;i<fields.length;i+=2) out.put((String)fields[i],fields[i+1]);
        return out;
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 4) throw new IllegalArgumentException("MAP FARC sample-manifest NEW-JSON");
        Path output = Path.of(args[3]).toAbsolutePath();
        Path local = Path.of("local-analysis").toRealPath();
        if (!output.getParent().toRealPath().startsWith(local) || Files.exists(output))
            throw new IllegalArgumentException("New output under local-analysis required");
        FileDB db = new FileDB(args[0]);
        FileArchive archive = new FileArchive(args[1]);
        List<Object> mapRows = new ArrayList<>(), archiveRows = new ArrayList<>(), samples = new ArrayList<>();
        Map<String,Fat> locations = new HashMap<>();
        for (FileDBRow r : db) mapRows.add(row("guid",guid(r.getGUID()),"path",r.getPath(),
            "sha1",hash(r.getSHA1()),"size",r.getSize(),"timestamp",r.getDate()));
        for (Fat f : archive) {
            archiveRows.add(row("sha1",hash(f.getSHA1()),"offset",f.getOffset(),"size",f.getSize()));
            locations.put(hash(f.getSHA1()),f);
        }
        JsonObject manifest = JsonParser.parseString(Files.readString(Path.of(args[2]))).getAsJsonObject();
        for (JsonElement sample : manifest.getAsJsonArray("samples")) {
            String sha = sample.getAsJsonObject().get("sha1").getAsString();
            Fat f = locations.get(sha);
            if (f == null || f.getSize() > 1048576 || f.getSize() < 12) continue;
            byte[] raw = f.extract(); // bounded, read-only FARC read
            if (raw[3] == 'e' || Arrays.equals(Arrays.copyOf(raw,4),new byte[]{'N','P','D',0})
                || Arrays.equals(Arrays.copyOf(raw,4),new byte[]{'S','C','E',0}))
                throw new IllegalArgumentException("Protected sample: stop");
            if (raw[3] != 'b') continue;
            String magic = new String(raw,0,4,java.nio.charset.StandardCharsets.US_ASCII);
            int depOffset = java.nio.ByteBuffer.wrap(raw,8,4).getInt();
            if (depOffset < 12 || depOffset > raw.length-4) throw new IOException("Invalid sample dependency range");
            int count = java.nio.ByteBuffer.wrap(raw,depOffset,4).getInt();
            if (count < 0 || count > 100000) throw new IOException("Sample count budget");
            SerializedResource resource = new SerializedResource(raw);
            Revision rev = resource.getRevision();
            List<Object> deps = new ArrayList<>();
            for (ResourceDescriptor dep : resource.getDependencies()) deps.add(row(
                "guid",guid(dep.getGUID()),"sha1",hash(dep.getSHA1()),"type_id",dep.getType().getValue()));
            java.lang.reflect.Field compressed = SerializedResource.class.getDeclaredField("isCompressed");
            compressed.setAccessible(true);
            samples.add(row("sha1",sha,"magic",magic,"type_id",resource.getResourceType().getValue(),
                "revision",rev.getHead(),"branch_id",Short.toUnsignedInt(rev.getBranchID()),
                "branch_revision",Short.toUnsignedInt(rev.getBranchRevision()),
                "compression_flags",Byte.toUnsignedInt(resource.getCompressionFlags()),
                "compressed",compressed.getBoolean(resource),"dependency_offset",depOffset,"dependencies",deps));
        }
        Gson gson = new GsonBuilder().serializeNulls().setPrettyPrinting().create();
        try (Writer writer = Files.newBufferedWriter(output, StandardOpenOption.CREATE_NEW)) {
            gson.toJson(row("toolkit_commit","d6a2ef80b5a09999f08f23a803b516096055e5c9",
                "map_rows",mapRows,"farc_rows",archiveRows,"samples",samples), writer);
        }
        System.out.printf("Toolkit oracle: %d MAP rows, %d FARC entries, %d clear binary samples%n",
            mapRows.size(),archiveRows.size(),samples.size());
    }
}

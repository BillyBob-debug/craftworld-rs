// Authored bounded external experiment. Provenance/conflicts: docs/plan-inventory-prefix.md.
// No full InventoryItemDetails/RPlan/Thing deserialization or reference resolution.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.types.data.Revision;
import cwlib.io.streams.MemoryInputStream;
import cwlib.io.serializer.Serializer;
import cwlib.structs.slot.SlotID;
import java.io.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public class PlanInventoryOracle {
    static int longEnd(byte[] b,int at,byte flags)throws IOException {
        if(flags==0){PlanOracle.need(b,at,8);return at+8;}
        if(flags!=7)throw new IOException("unsupported flags");
        for(int i=0;i<10;i++) {
            PlanOracle.need(b,at+i,1);int v=b[at+i]&255;
            if(i==9&&v>1)throw new IOException("u64 overflow");
            if((v&128)==0)return at+i+1;
        }
        throw new IOException("u64 overflow");
    }
    static Map<String,Object> prefix(byte[] clear,int start,int revision,byte flags)throws Exception {
        if(revision<893||revision>951||clear.length>PlanOracle.EXPANDED)throw new IOException("unsupported inventory prefix revision/budget");
        PlanOracle.need(clear,start,0);MemoryInputStream s=new MemoryInputStream(clear,flags);
        s.seek(start,MemoryInputStream.SeekMode.Begin);int dateEnd=longEnd(clear,start,flags);
        long encoded=s.u64();if(s.getOffset()!=dateEnd)throw new IOException("date offset disagreement");
        s.seek(start,MemoryInputStream.SeekMode.Begin);long readerDate=s.s64();
        if(s.getOffset()!=dateEnd)throw new IOException("signed date offset disagreement");
        long expectedDate=flags==0?encoded:((encoded>>>1)^-(encoded&1));
        int slotStart=s.getOffset();long slotType=PlanOracle.integer(s,flags),slotNumber=PlanOracle.integer(s,flags);
        int slotEnd=s.getOffset();
        MemoryInputStream probe=new MemoryInputStream(clear,flags);probe.seek(slotStart,MemoryInputStream.SeekMode.Begin);
        SlotID slot=new Serializer(probe,new Revision(revision)).struct(null,SlotID.class);
        if(probe.getOffset()!=slotEnd||slot.slotNumber!=slotNumber||
            (slot.slotType!=null&&Integer.toUnsignedLong(slot.slotType.getValue())!=slotType))throw new IOException("SlotID reader disagreement");
        long sound=PlanOracle.integer(s,flags),colour=PlanOracle.integer(s,flags),type=PlanOracle.integer(s,flags),
            subType=PlanOracle.integer(s,flags),title=PlanOracle.integer(s,flags),description=PlanOracle.integer(s,flags);
        Map<String,Object> metadata=TextureOracle.row("date_added_encoded_u64_hex",String.format(Locale.ROOT,"%016x",encoded),
            "slot_type_raw_u32",slotType,"slot_number_raw_u32",slotNumber,"highlight_sound_guid_raw_u32",sound,
            "colour_raw_u32",colour,"type_flags_raw_u32",type,"sub_type_raw_u32",subType,
            "title_key_raw_u32",title,"description_key_raw_u32",description,
            "inventory_prefix_start",start,"parsed_inventory_prefix_bytes",s.getOffset()-start,
            "inventory_prefix_end",s.getOffset(),"unparsed_after_inventory_prefix",clear.length-s.getOffset());
        return TextureOracle.row("metadata",metadata,"reader_slot_status",slot.slotType==null?"unknown_enum_raw_retained":"class_fields_equal",
            "reader_date_i64_decimal",Long.toString(readerDate),"standard_date_i64_decimal",Long.toString(expectedDate),
            "reader_date_status",readerDate==expectedDate?"equal":"known_arithmetic_shift_conflict");
    }
    static void longEncoded(ByteArrayOutputStream b,long value,byte flags)throws IOException {
        if(flags==0){new DataOutputStream(b).writeLong(value);return;}
        do{int v=(int)(value&127);value>>>=7;b.write(v|(value!=0?128:0));}while(value!=0);
    }
    static byte[] synthetic(byte flags,long date,long slotType)throws IOException {
        ByteArrayOutputStream b=new ByteArrayOutputStream();longEncoded(b,date,flags);
        for(long v:new long[]{slotType,0xffffffffL,0,0xff00ff00L,0xffffffffL,0x80000000L,42,0})PlanOracle.encoded(b,v,flags);
        b.write(new byte[]{(byte)255,(byte)255,(byte)255,(byte)255}); // Stop before unknown recursive tag.
        return b.toByteArray();
    }
    static void selfTest()throws Exception {
        for(byte flags:new byte[]{0,7})for(long date:new long[]{0,1,2,Long.MAX_VALUE,Long.MIN_VALUE,-2,-1})
            for(long slotType:new long[]{0,0xffffffffL}) {
                byte[] raw=synthetic(flags,date,slotType);Map<String,Object> r=prefix(raw,0,919,flags);
                Map<?,?> m=(Map<?,?>)r.get("metadata");int end=(int)m.get("inventory_prefix_end");
                if(!m.get("slot_type_raw_u32").equals(slotType)||!m.get("slot_number_raw_u32").equals(0xffffffffL)||
                    !m.get("type_flags_raw_u32").equals(0xffffffffL)||!m.get("unparsed_after_inventory_prefix").equals(4))throw new AssertionError("raw prefix");
                boolean conflict=flags==7&&date<0;
                if(!r.get("reader_date_status").equals(conflict?"known_arithmetic_shift_conflict":"equal"))throw new AssertionError("date conflict changed");
                for(int n=0;n<end;n++) {
                    try{prefix(Arrays.copyOf(raw,n),0,919,flags);}catch(IOException expected){continue;}
                    throw new AssertionError("accepted prefix truncation "+n);
                }
            }
        byte[] overflow=new byte[10];Arrays.fill(overflow,(byte)255);overflow[9]=2;
        try{prefix(overflow,0,919,(byte)7);throw new AssertionError("u64 overflow");}catch(IOException expected){}
        System.out.println("Inventory prefix guards/raw fields/SlotID/s64 conflict/truncations passed");
    }
    public static void main(String[] args)throws Exception {
        if(args.length==1&&args[0].equals("--self-test")){selfTest();return;}
        if(args.length!=3)throw new IllegalArgumentException("FARC MANIFEST NEW-JSON");
        Path output=Path.of(args[2]).toAbsolutePath();
        if(!output.toString().endsWith(".json")||Files.exists(output)||
            !output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored JSON required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        if(!manifest.get("toolkit_commit").getAsString().equals(PlanOracle.PIN)||
            !manifest.get("scope").getAsString().equals("clear_plan_prefix_revision_893_951_branch_zero"))throw new IOException("manifest pin/scope differs");
        JsonArray samples=manifest.getAsJsonArray("samples");if(samples.size()>48)throw new IOException("sample budget");
        List<Object> rows=new ArrayList<>();long total=0,deadline=System.nanoTime()+60_000_000_000L;
        try(RandomAccessFile f=new RandomAccessFile(args[0],"r")) {
            for(JsonElement item:samples) {
                if(Thread.currentThread().isInterrupted()||System.nanoTime()>deadline)throw new IOException("oracle cancelled/time budget");
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();Map<String,Object> row=TextureOracle.row("sha1",sha);rows.add(row);
                try {
                    long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(!q.get("source_path").getAsString().equals("USRDIR/DATA.FARC")||at<0||size<18||size>2*1024*1024||
                        at>f.length()-size||total+size>64*1024*1024)throw new IOException("source/read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(raw)).equals(sha))throw new IOException("identifier mismatch");
                    Map<String,Object> plan=PlanOracle.inspect(raw);row.put("plan",plan);
                    Map<?,?> p=(Map<?,?>)plan.get("prefix");
                    // Existing adapter fully preflighted this ordinary clear wrapper;
                    // second constructor is bounded, no additional archive/source read.
                    SerializedResource resource=new SerializedResource(raw);
                    row.put("inventory",prefix(resource.getStream().getBuffer(),(int)p.get("inventory_tail_start"),
                        (int)p.get("revision"),((Integer)plan.get("compression_flags")).byteValue()));
                    row.put("status","parsed_inventory_prefix");
                }catch(Exception e){row.putAll(TextureOracle.row("status","rejected","reason",e.getClass().getSimpleName()+": "+e.getMessage()));}
            }
        }
        Files.writeString(output,new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(TextureOracle.row(
            "schema_version",1,"toolkit_commit",PlanOracle.PIN,"scope","plan_inventory_scalar_prefix_no_references",
            "sample_bytes",total,"samples",rows)),StandardOpenOption.CREATE_NEW);
        System.out.println("Plan inventory oracle: "+rows.size()+" samples, "+total+" archived bytes");
    }
}

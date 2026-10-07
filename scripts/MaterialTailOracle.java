// Authored guarded external-reader adapter; no product parser or recovered source.
// CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
// RGfxMaterial::serialize, Revisions::GFXMATERIAL_SOUND_ENUM/PARAMETER_ANIMATIONS,
// Serializer::array, MaterialParameterAnimation::serialize. PS3 heads920–951 only.
// https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java
// Per-rule evidence, conflicts and tests: docs/material-tail.md. No key/asset export.
import cwlib.io.streams.MemoryInputStream;
import cwlib.io.serializer.Serializer;
import cwlib.types.data.Revision;
import cwlib.enums.Revisions;
import cwlib.structs.gmat.MaterialParameterAnimation;
import java.io.*;
import java.nio.*;
import java.util.*;
import java.util.function.BooleanSupplier;

public class MaterialTailOracle {
    static final int MAX_ANIMATIONS=128, MAX_TAIL_BYTES=2*1024*1024;
    static void check(BooleanSupplier cancelled)throws IOException {
        if(Thread.currentThread().isInterrupted()||cancelled.getAsBoolean())throw new IOException("tail cancelled/deadline");
    }
    static Map<String,Object> tail(MemoryInputStream s,Revision revision,BooleanSupplier cancelled)throws Exception {
        check(cancelled);
        int head=revision.getVersion();
        if(head<920||head>951||revision.getSubVersion()!=0||revision.isVita()||revision.isToolkit())
            throw new IOException("unsupported tail revision");
        int start=s.getOffset();
        if(start<0||start>s.getBuffer().length||s.getBuffer().length-start>MAX_TAIL_BYTES)
            throw new IOException("tail byte budget");
        // These thresholds are library constants, not inferred from remaining bytes.
        if(head<Revisions.GFXMATERIAL_SOUND_ENUM||head<Revisions.PARAMETER_ANIMATIONS)
            throw new IOException("tail field revision");
        long sound=MaterialOracle.unsigned(s.i32()); MaterialGraphOracle.bounds(s);
        int animationStart=s.getOffset(),count=MaterialOracle.count(s,MAX_ANIMATIONS);
        List<Map<String,Object>> animations=new ArrayList<>();
        for(int i=0;i<count;i++) {
            check(cancelled);
            animations.add(MaterialGraphOracle.animation(s));
        }
        MaterialGraphOracle.bounds(s); check(cancelled);
        // Invoke only the isolated bounded array reader, never full RGfxMaterial.
        MemoryInputStream isolated=new MemoryInputStream(Arrays.copyOfRange(s.getBuffer(),animationStart,s.getOffset()),s.getCompressionFlags());
        MaterialParameterAnimation[] read=new Serializer(isolated,revision).array(null,MaterialParameterAnimation.class);
        if(isolated.getOffset()!=isolated.getBuffer().length||read.length!=count)
            throw new IOException("parameter animation array boundary disagreement");
        for(int i=0;i<count;i++) {
            check(cancelled);
            if((read[i].keys==null?0:read[i].keys.length)!=(int)animations.get(i).get("derived_key_count"))
                throw new IOException("parameter animation key count disagreement");
            MaterialGraphOracle.compareAnimation(read[i],animations.get(i));
        }
        return TextureOracle.row("sound_enum_raw",sound,"parameter_animation_count",count,
            "parameter_animations",animations,"tail_start",start,"parsed_tail_bytes",s.getOffset()-start,
            "unparsed_bytes",s.getBuffer().length-s.getOffset());
    }
    static byte[] recipe(boolean compact)throws Exception {
        ByteArrayOutputStream b=new ByteArrayOutputStream();
        MaterialGraphOracle.integer(b,-1,compact); MaterialGraphOracle.integer(b,2,compact);
        for(int i=0;i<2;i++) {
            b.write(ByteBuffer.allocate(16).putInt(0x7fc00001).putInt(0x80000000).putLong(0).array());
            MaterialGraphOracle.integer(b,i==0?0:4,compact);
            if(i!=0)b.write(ByteBuffer.allocate(16).putInt(0x3f800000).putInt(0x40000000).putInt(0x40400000).putInt(0x40800000).array());
            MaterialGraphOracle.integer(b,3,compact); b.write(new byte[]{65,0,90}); b.write(i==0?255:5);
        }
        return b.toByteArray();
    }
    static void rejected(byte[] bytes,boolean compact)throws Exception {
        try { tail(new MemoryInputStream(bytes,(byte)(compact?7:0)),new Revision(950),()->false); }
        catch(Exception expected) { return; }
        throw new AssertionError("invalid tail accepted");
    }
    static void selfTest()throws Exception {
        for(boolean compact:new boolean[]{false,true}) {
            byte[] bytes=recipe(compact);
            Map<String,Object> r=tail(new MemoryInputStream(bytes,(byte)(compact?7:0)),new Revision(950),()->false);
            if(!r.get("sound_enum_raw").equals(0xffffffffL)||!r.get("parameter_animation_count").equals(2)||!r.get("unparsed_bytes").equals(0))
                throw new AssertionError("tail raw fields/counts");
            List<?> a=(List<?>)r.get("parameter_animations");
            if(!((Map<?,?>)a.get(1)).get("derived_key_count").equals(2))throw new AssertionError("tail keys");
            for(int n=0;n<bytes.length;n++)rejected(Arrays.copyOf(bytes,n),compact);
            byte[] extra=Arrays.copyOf(bytes,bytes.length+3);
            if(!tail(new MemoryInputStream(extra,(byte)(compact?7:0)),new Revision(950),()->false).get("unparsed_bytes").equals(3))
                throw new AssertionError("unknown tail lost");
            ByteArrayOutputStream bad=new ByteArrayOutputStream(); MaterialGraphOracle.integer(bad,0,compact); MaterialGraphOracle.integer(bad,129,compact); rejected(bad.toByteArray(),compact);
            bad.reset(); MaterialGraphOracle.integer(bad,0,compact); MaterialGraphOracle.integer(bad,1,compact); bad.write(new byte[16]); MaterialGraphOracle.integer(bad,4097,compact); rejected(bad.toByteArray(),compact);
            bad.reset(); MaterialGraphOracle.integer(bad,0,compact); MaterialGraphOracle.integer(bad,1,compact); bad.write(new byte[16]); MaterialGraphOracle.integer(bad,0,compact); MaterialGraphOracle.integer(bad,65,compact); rejected(bad.toByteArray(),compact);
            int[] calls={0};
            try { tail(new MemoryInputStream(bytes,(byte)(compact?7:0)),new Revision(950),()->++calls[0]>=3); throw new AssertionError("cancel accepted"); }
            catch(IOException expected) { if(!expected.getMessage().contains("cancelled"))throw expected; }
            byte[] mask=bytes.clone(); mask[mask.length-1]=0; rejected(mask,compact);
            mask[mask.length-1]=7; rejected(mask,compact);
            bad.reset(); MaterialGraphOracle.integer(bad,0,compact); MaterialGraphOracle.integer(bad,0,compact);
            Map<String,Object> empty=tail(new MemoryInputStream(bad.toByteArray(),(byte)(compact?7:0)),new Revision(950),()->false);
            if(!empty.get("parameter_animation_count").equals(0)||!empty.get("unparsed_bytes").equals(0))throw new AssertionError("empty array");
        }
        rejected(new byte[MAX_TAIL_BYTES+1],false);
        try { tail(new MemoryInputStream(recipe(false)),new Revision(919),()->false); throw new AssertionError("revision accepted"); }
        catch(IOException expected) { if(!expected.getMessage().contains("revision"))throw expected; }
        System.out.println("Synthetic tail fixed/packed/raw fields/array/truncation/budget/cancellation guards passed");
    }
}

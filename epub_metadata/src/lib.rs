//! This crate collates a number of standardised terms for book and ebook metadata and makes them available to use
//! as Rust enums.
//! It is primarily a library structure to let tagged metadata be used across different crates and includes
//! little functionality of its own.

#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_debug_implementations)]
#![deny(unused_results)]
#![deny(variant_size_differences)]

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::str::FromStr;

// Sources:
// dc          dublin core elements
// dcterms     http://purl.org/dc/terms/
// marc    http://id.loc.gov/vocabulary/
// onix    http://www.editeur.org/ONIX/book/codelists/current.html#
// structural semantics https://idpf.github.io/epub-vocabs/structure/

/// The role of a contributor, either as an ONIX or MARC code
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum ContributorRole {
    /// An onix code
    Onix(OnixContributorCode),
    /// A marc code
    Marc(MarcRelator),
}

impl FromStr for ContributorRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let as_marc_relator = s.parse::<MarcRelator>();
        if let Ok(m) = as_marc_relator {
            return Ok(ContributorRole::Marc(m));
        }
        if let Ok(o) = s.parse::<OnixContributorCode>() {
            return Ok(ContributorRole::Onix(o));
        }
        Err(())
    }
}

impl TryFrom<ContributorRole> for MarcRelator {
    type Error = ();

    fn try_from(c: ContributorRole) -> Result<Self, Self::Error> {
        match c {
            ContributorRole::Marc(m) => Ok(m),
            ContributorRole::Onix(o) => {
                if let Some(m) = o.map_code() {
                    Ok(m)
                } else {
                    Err(())
                }
            }
        }
    }
}

// https://idpf.github.io/epub-vocabs/structure/#sections
/// Document partitions
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum DocumentPartition {
    /// the cover of a document
    Cover,
    /// Frontmatter
    Frontmatter,
    /// Bodymatter
    Bodymatter,
    /// Backmatter
    Backmatter,
}

/// A mainmatter division of a document
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum DocumentDivision {
    Volume,
    Part,
    Chapter,
}

///
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum DocumentSectionOrComponent {
    Abstract,
    Foreword,
    Preface,
    Prologue,
    Introduction,
    Preamble,
    Conclusion,
    Epilogue,
    Afterword,
    Epigraph,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum DocumentNavigation {
    Toc,
    TocBrief,
    Landmarks,
    Loa,
    Loi,
    Lot,
    Lov,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum DocumentReferenceSection {
    Appendix,
    Colophon,
    Credits,
    Keywords,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum PreliminarySection {
    Titlepage,
    Halftitlepage,
    CopyrightPage,
    SeriesPage,
    Acknowledgements,
    Imprint,
    Imprimatur,
    Contributors,
    OtherCredits,
    Errata,
    Dedication,
    RevisionHistory,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum TitlesAndHeadings {
    Halftitle,
    Fulltitle,
    Covertitle,
    Title,
    Subtitle,
    Bridgehead,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Notes {
    Footnote,
    Endnote,
    Footnotes,
    Endnotes,
    NoteRef,
}

/// Marc relators (<http://id.loc.gov/vocabulary/relators.html/>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MarcRelator {
    /// Abridger: A person, family, or organization contributing to a resource by shortening or condensing the original work but leaving the nature and content of the original work substantially unchanged. For substantial modifications that result in the creation of a new work, see author
    Abr,
    /// Actor: A performer contributing to an expression of a work by acting as a cast member or player in a musical or dramatic presentation, etc.
    Act,
    /// Adapter: A person or organization who 1) reworks a musical composition, usually for a different medium, or 2) rewrites novels or stories for motion pictures or other audiovisual medium.
    Adp,
    /// Addressee: A person, family, or organization to whom the correspondence in a work is addressed
    Rcp,
    /// Analyst: A person or organization that reviews, examines and interprets data or information in a specific area
    Anl,
    /// Animator: A person contributing to a moving image work or computer program by giving apparent movement to inanimate objects or drawings. For the creator of the drawings that are animated, see artist
    Anm,
    /// Annotator: A person who makes manuscript annotations on an item
    Ann,
    /// Appellant: A person or organization who appeals a lower court's decision
    Apl,
    /// Appellee: A person or organization against whom an appeal is taken
    Ape,
    /// Applicant: A person or organization responsible for the submission of an application or who is named as eligible for the results of the processing of the application (e.g., bestowing of rights, reward, title, position)
    App,
    /// Architect: A person, family, or organization responsible for creating an architectural design, including a pictorial representation intended to show how a building, etc., will look when completed. It also oversees the construction of structures
    Arc,
    /// Arranger: A person, family, or organization contributing to a musical work by rewriting the composition for a medium of performance different from that for which the work was originally intended, or modifying the work for the same medium of performance, etc., such that the musical substance of the original composition remains essentially unchanged. For extensive modification that effectively results in the creation of a new musical work, see composer
    Arr,
    /// Art copyist: A person (e.g., a painter or sculptor) who makes copies of works of visual art
    Acp,
    /// Art director: A person contributing to a motion picture or television production by overseeing the artists and craftspeople who build the sets
    Adi,
    /// Artist: A person, family, or organization responsible for creating a work by conceiving, and implementing, an original graphic design, drawing, painting, etc. For book illustrators, prefer Illustrator [ill]
    Art,
    /// Artistic director: A person responsible for controlling the development of the artistic style of an entire production, including the choice of works to be presented and selection of senior production staff
    Ard,
    /// Assignee: A person or organization to whom a license for printing or publishing has been transferred
    Asg,
    /// Associated name: A person or organization associated with or found in an item or collection, which cannot be determined to be that of a Former owner [fmo] or other designated relationship indicative of provenance
    Asn,
    /// Attributed name: An author, artist, etc., relating him/her to a resource for which there is or once was substantial authority for designating that person as author, creator, etc. of the work
    Att,
    /// Auctioneer: A person or organization in charge of the estimation and public auctioning of goods, particularly books, artistic works, etc.
    Auc,
    /// Author: A person, family, or organization responsible for creating a work that is primarily textual in content, regardless of media type (e.g., printed text, spoken word, electronic text, tactile text) or genre (e.g., poems, novels, screenplays, blogs). Use also for persons, etc., creating a new work by paraphrasing, rewriting, or adapting works by another creator such that the modification has substantially changed the nature and content of the original or changed the medium of expression
    Aut,
    /// Author in quotations or text abstracts: A person or organization whose work is largely quoted or extracted in works to which he or she did not contribute directly. Such quotations are found particularly in exhibition catalogs, collections of photographs, etc.
    Aqt,
    /// Author of afterword, colophon, etc.: A person or organization responsible for an afterword, postface, colophon, etc. but who is not the chief author of a work
    Aft,
    /// Author of dialog: A person or organization responsible for the dialog or spoken commentary for a screenplay or sound recording
    Aud,
    /// Author of introduction, etc.: A person or organization responsible for an introduction, preface, foreword, or other critical introductory matter, but who is not the chief author
    Aui,
    /// Autographer: A person whose manuscript signature appears on an item
    Ato,
    /// Bibliographic antecedent: A person or organization responsible for a resource upon which the resource represented by the bibliographic description is based. This may be appropriate for adaptations, sequels, continuations, indexes, etc.
    Ant,
    /// Binder: A person who binds an item
    Bnd,
    /// Binding designer: A person or organization responsible for the binding design of a book, including the type of binding, the type of materials used, and any decorative aspects of the binding
    Bdd,
    /// Blurb writer: A person or organization responsible for writing a commendation or testimonial for a work, which appears on or within the publication itself, frequently on the back or dust jacket of print publications or on advertising material for all media
    Blw,
    /// Book designer: A person or organization involved in manufacturing a manifestation by being responsible for the entire graphic design of a book, including arrangement of type and illustration, choice of materials, and process used
    Bkd,
    /// Book producer: A person or organization responsible for the production of books and other print media
    Bkp,
    /// Bookjacket designer: A person or organization responsible for the design of flexible covers designed for or published with a book, including the type of materials used, and any decorative aspects of the bookjacket
    Bjd,
    /// Bookplate designer: A person or organization responsible for the design of a book owner's identification label that is most commonly pasted to the inside front cover of a book
    Bpd,
    /// Bookseller: A person or organization who makes books and other bibliographic materials available for purchase. Interest in the materials is primarily lucrative
    Bsl,
    /// Braille embosser: A person, family, or organization involved in manufacturing a resource by embossing Braille cells using a stylus, special embossing printer, or other device
    Brl,
    /// Broadcaster: A person, family, or organization involved in broadcasting a resource to an audience via radio, television, webcast, etc.
    Brd,
    /// Calligrapher: A person or organization who writes in an artistic hand, usually as a copyist and or engrosser
    Cll,
    /// Cartographer: A person, family, or organization responsible for creating a map, atlas, globe, or other cartographic work
    Ctg,
    /// Caster: A person, family, or organization involved in manufacturing a resource by pouring a liquid or molten substance into a mold and leaving it to solidify to take the shape of the mold
    Cas,
    /// Censor: A person or organization who examines bibliographic resources for the purpose of suppressing parts deemed objectionable on moral, political, military, or other grounds
    Cns,
    /// Choreographer: A person responsible for creating or contributing to a work of movement
    Chr,
    /// Cinematographer: A person in charge of photographing a motion picture, who plans the technical aspets of lighting and photographing of scenes, and often assists the director in the choice of angles, camera setups, and lighting moods. He or she may also supervise the further processing of filmed material up to the completion of the work print. Cinematographer is also referred to as director of photography. Do not confuse with videographer
    Cng,
    /// Client: A person or organization for whom another person or organization is acting
    Cli,
    /// Collection registrar: A curator who lists or inventories the items in an aggregate work such as a collection of items or works
    Cor,
    /// Collector: A curator who brings together items from various sources that are then arranged, described, and cataloged as a collection. A collector is neither the creator of the material nor a person to whom manuscripts in the collection may have been addressed
    Col,
    /// Collotyper: A person, family, or organization involved in manufacturing a manifestation of photographic prints from film or other colloid that has ink-receptive and ink-repellent surfaces
    Clt,
    /// Colorist: A person or organization responsible for applying color to drawings, prints, photographs, maps, moving images, etc
    Clr,
    /// Commentator: A performer contributing to a work by providing interpretation, analysis, or a discussion of the subject matter on a recording, film, or other audiovisual medium
    Cmm,
    /// Commentator for written text: A person or organization responsible for the commentary or explanatory notes about a text. For the writer of manuscript annotations in a printed book, use Annotator
    Cwt,
    /// Compiler: A person, family, or organization responsible for creating a new work (e.g., a bibliography, a directory) through the act of compilation, e.g., selecting, arranging, aggregating, and editing data, information, etc
    Com,
    /// Complainant: A person or organization who applies to the courts for redress, usually in an equity proceeding
    Cpl,
    /// Complainant-appellant: A complainant who takes an appeal from one court or jurisdiction to another to reverse the judgment, usually in an equity proceeding
    Cpt,
    /// Complainant-appellee: A complainant against whom an appeal is taken from one court or jurisdiction to another to reverse the judgment, usually in an equity proceeding
    Cpe,
    /// Composer: A person, family, or organization responsible for creating or contributing to a musical resource by adding music to a work that originally lacked it or supplements it
    Cmp,
    /// Compositor: A person or organization responsible for the creation of metal slug, or molds made of other materials, used to produce the text and images in printed matter
    Cmt,
    /// Conceptor: A person or organization responsible for the original idea on which a work is based, this includes the scientific author of an audio-visual item and the conceptor of an advertisement
    Ccp,
    /// Conductor: A performer contributing to a musical resource by leading a performing group (orchestra, chorus, opera, etc.) in a musical or dramatic presentation, etc.
    Cnd,
    /// Conservator: A person or organization responsible for documenting, preserving, or treating printed or manuscript material, works of art, artifacts, or other media
    Con,
    /// Consultant: A person or organization relevant to a resource, who is called upon for professional advice or services in a specialized field of knowledge or training
    Csl,
    /// Consultant to a project: A person or organization relevant to a resource, who is engaged specifically to provide an intellectual overview of a strategic or operational task and by analysis, specification, or instruction, to create or propose a cost-effective course of action or solution
    Csp,
    /// Contestant: A person(s) or organization who opposes, resists, or disputes, in a court of law, a claim, decision, result, etc.
    Cos,
    /// Contestant-appellant: A contestant who takes an appeal from one court of law or jurisdiction to another to reverse the judgment
    Cot,
    /// Contestant-appellee: A contestant against whom an appeal is taken from one court of law or jurisdiction to another to reverse the judgment
    Coe,
    /// Contestee: A person(s) or organization defending a claim, decision, result, etc. being opposed, resisted, or disputed in a court of law
    Cts,
    /// Contestee-appellant: A contestee who takes an appeal from one court or jurisdiction to another to reverse the judgment
    Ctt,
    /// Contestee-appellee: A contestee against whom an appeal is taken from one court or jurisdiction to another to reverse the judgment
    Cte,
    /// Contractor: A person or organization relevant to a resource, who enters into a contract with another person or organization to perform a specific
    Ctr,
    /// Contributor: A person, family or organization responsible for making contributions to the resource. This includes those whose work has been contributed to a larger work, such as an anthology, serial publication, or other compilation of individual works. If a more specific role is available, prefer that, e.g. editor, compiler, illustrator
    Ctb,
    /// Copyright claimant: A person or organization listed as a copyright owner at the time of registration. Copyright can be granted or later transferred to another person or organization, at which time the claimant becomes the copyright holder
    Cpc,
    /// Copyright holder: A person or organization to whom copy and legal rights have been granted or transferred for the intellectual content of a work. The copyright holder, although not necessarily the creator of the work, usually has the exclusive right to benefit financially from the sale and use of the work to which the associated copyright protection applies
    Cph,
    /// Corrector: A person or organization who is a corrector of manuscripts, such as the scriptorium official who corrected the work of a scribe. For printed matter, use Proofreader
    Crr,
    /// Correspondent: A person or organization who was either the writer or recipient of a letter or other communication
    Crp,
    /// Costume designer: A person, family, or organization that designs the costumes for a moving image production or for a musical or dramatic presentation or entertainment
    Cst,
    /// Court governed: A court governed by court rules, regardless of their official nature (e.g., laws, administrative regulations)
    Cou,
    /// Court reporter: A person, family, or organization contributing to a resource by preparing a court's opinions for publication
    Crt,
    /// Cover designer: A person or organization responsible for the graphic design of a book cover, album cover, slipcase, box, container, etc. For a person or organization responsible for the graphic design of an entire book, use Book designer; for book jackets, use Bookjacket designer
    Cov,
    /// Creator: A person or organization responsible for the intellectual or artistic content of a resource
    Cre,
    /// Curator: A person, family, or organization conceiving, aggregating, and/or organizing an exhibition, collection, or other item
    Cur,
    /// Dancer: A performer who dances in a musical, dramatic, etc., presentation
    Dnc,
    /// Data contributor: A person or organization that submits data for inclusion in a database or other collection of data
    Dtc,
    /// Data manager: A person or organization responsible for managing databases or other data sources
    Dtm,
    /// Dedicatee: A person, family, or organization to whom a resource is dedicated
    Dte,
    /// Dedicator: A person who writes a dedication, which may be a formal statement or in epistolary or verse form
    Dto,
    /// Defendant: A person or organization who is accused in a criminal proceeding or sued in a civil proceeding
    Dfd,
    /// Defendant-appellant: A defendant who takes an appeal from one court or jurisdiction to another to reverse the judgment, usually in a legal action
    Dft,
    /// Defendant-appellee: A defendant against whom an appeal is taken from one court or jurisdiction to another to reverse the judgment, usually in a legal action
    Dfe,
    /// Degree granting institution: A organization granting an academic degree
    Dgg,
    /// Degree supervisor: A person overseeing a higher level academic degree
    Dgs,
    /// Delineator: A person or organization executing technical drawings from others' designs
    Dln,
    /// Depicted: An entity depicted or portrayed in a work, particularly in a work of art
    Dpc,
    /// Depositor: A current owner of an item who deposited the item into the custody of another person, family, or organization, while still retaining ownership
    Dpt,
    /// Designer: A person, family, or organization responsible for creating a design for an object
    Dsr,
    /// Director: A person responsible for the general management and supervision of a filmed performance, a radio or television program, etc.
    Drt,
    /// Dissertant: A person who presents a thesis for a university or higher-level educational degree
    Dis,
    /// Distribution place: A place from which a resource, e.g., a serial, is distributed
    Dbp,
    /// Distributor: A person or organization that has exclusive or shared marketing rights for a resource
    Dst,
    /// Donor: A former owner of an item who donated that item to another owner
    Dnr,
    /// Draftsman: A person, family, or organization contributing to a resource by an architect, inventor, etc., by making detailed plans or drawings for buildings, ships, aircraft, machines, objects, etc
    Drm,
    /// Dubious author: A person or organization to which authorship has been dubiously or incorrectly ascribed
    Dub,
    /// Editor: A person, family, or organization contributing to a resource by revising or elucidating the content, e.g., adding an introduction, notes, or other critical matter. An editor may also prepare a resource for production, publication, or distribution. For major revisions, adaptations, etc., that substantially change the nature and content of the original work, resulting in a new work, see author
    Edt,
    /// Editor of compilation: A person, family, or organization contributing to a collective or aggregate work by selecting and putting together works, or parts of works, by one or more creators. For compilations of data, information, etc., that result in new works, see compiler
    Edc,
    /// Editor of moving image work: A person, family, or organization responsible for assembling, arranging, and trimming film, video, or other moving image formats, including both visual and audio aspects
    Edm,
    /// Electrician: A person responsible for setting up a lighting rig and focusing the lights for a production, and running the lighting at a performance
    Elg,
    /// Electrotyper: A person or organization who creates a duplicate printing surface by pressure molding and electrodepositing of metal that is then backed up with lead for printing
    Elt,
    /// Enacting jurisdiction: A jurisdiction enacting a law, regulation, constitution, court rule, etc.
    Enj,
    /// Engineer: A person or organization that is responsible for technical planning and design, particularly with construction
    Eng,
    /// Engraver: A person or organization who cuts letters, figures, etc. on a surface, such as a wooden or metal plate used for printing
    Egr,
    /// Etcher: A person or organization who produces text or images for printing by subjecting metal, glass, or some other surface to acid or the corrosive action of some other substance
    Etr,
    /// Event place: A place where an event such as a conference or a concert took place
    Evp,
    /// Expert: A person or organization in charge of the description and appraisal of the value of goods, particularly rare items, works of art, etc.
    Exp,
    /// Facsimilist: A person or organization that executed the facsimile
    Fac,
    /// Field director: A person or organization that manages or supervises the work done to collect raw data or do research in an actual setting or environment (typically applies to the natural and social sciences)
    Fld,
    /// Film director: A director responsible for the general management and supervision of a filmed performance
    Fmd,
    /// Film distributor: A person, family, or organization involved in distributing a moving image resource to theatres or other distribution channels
    Fds,
    /// Film editor: A person who, following the script and in creative cooperation with the Director, selects, arranges, and assembles the filmed material, controls the synchronization of picture and sound, and participates in other post-production tasks such as sound mixing and visual effects processing. Today, picture editing is often performed digitally.
    Flm,
    /// Film producer: A producer responsible for most of the business aspects of a film
    Fmp,
    /// Filmmaker: A person, family or organization responsible for creating an independent or personal film. A filmmaker is individually responsible for the conception and execution of all aspects of the film
    Fmk,
    /// First party: A person or organization who is identified as the only party or the party of the first party. In the case of transfer of rights, this is the assignor, transferor, licensor, grantor, etc. Multiple parties can be named jointly as the first party
    Fpy,
    /// Forger: A person or organization who makes or imitates something of value or importance, especially with the intent to defraud
    Frg,
    /// Former owner: A person, family, or organization formerly having legal possession of an item
    Fmo,
    /// Funder: A person or organization that furnished financial support for the production of the work
    Fnd,
    /// Geographic information specialist: A person responsible for geographic information system (GIS) development and integration with global positioning system data
    Gis,
    /// Honoree: A person, family, or organization honored by a work or item (e.g., the honoree of a festschrift, a person to whom a copy is presented)
    Hnr,
    /// Host: A performer contributing to a resource by leading a program (often broadcast) that includes other guests, performers, etc. (e.g., talk show host)
    Hst,
    /// Host institution: An organization hosting the event, exhibit, conference, etc., which gave rise to a resource, but having little or no responsibility for the content of the resource
    His,
    /// Illuminator: A person providing decoration to a specific item using precious metals or color, often with elaborate designs and motifs
    Ilu,
    /// Illustrator: A person, family, or organization contributing to a resource by supplementing the primary content with drawings, diagrams, photographs, etc. If the work is primarily the artistic content created by this entity, use artist or photographer
    Ill,
    /// Inscriber: A person who has written a statement of dedication or gift
    Ins,
    /// Instrumentalist: A performer contributing to a resource by playing a musical instrument
    Itr,
    /// Interviewee: A person, family or organization responsible for creating or contributing to a resource by responding to an interviewer, usually a reporter, pollster, or some other information gathering agent
    Ive,
    /// Interviewer: A person, family, or organization responsible for creating or contributing to a resource by acting as an interviewer, reporter, pollster, or some other information gathering agent
    Ivr,
    /// Inventor: A person, family, or organization responsible for creating a new device or process
    Inv,
    /// Issuing body: A person, family or organization issuing a work, such as an official organ of the body
    Isb,
    /// Judge: A person who hears and decides on legal matters in court.
    Jud,
    /// Jurisdiction governed: A jurisdiction governed by a law, regulation, etc., that was enacted by another jurisdiction
    Jug,
    /// Laboratory: An organization that provides scientific analyses of material samples
    Lbr,
    /// Laboratory director: A person or organization that manages or supervises work done in a controlled setting or environment
    Ldr,
    /// Landscape architect: An architect responsible for creating landscape works. This work involves coordinating the arrangement of existing and proposed land features and structures
    Lsa,
    /// Lead: A person or organization that takes primary responsibility for a particular activity or endeavor. May be combined with another relator term or code to show the greater importance this person or organization has regarding that particular role. If more than one relator is assigned to a heading, use the Lead relator only if it applies to all the relators
    Led,
    /// Lender: A person or organization permitting the temporary use of a book, manuscript, etc., such as for photocopying or microfilming
    Len,
    /// Libelant: A person or organization who files a libel in an ecclesiastical or admiralty case
    Lil,
    /// Libelant-appellant: A libelant who takes an appeal from one ecclesiastical court or admiralty to another to reverse the judgment
    Lit,
    /// Libelant-appellee: A libelant against whom an appeal is taken from one ecclesiastical court or admiralty to another to reverse the judgment
    Lie,
    /// Libelee: A person or organization against whom a libel has been filed in an ecclesiastical court or admiralty
    Lel,
    /// Libelee-appellant: A libelee who takes an appeal from one ecclesiastical court or admiralty to another to reverse the judgment
    Let,
    /// Libelee-appellee: A libelee against whom an appeal is taken from one ecclesiastical court or admiralty to another to reverse the judgment
    Lee,
    /// Librettist: An author of a libretto of an opera or other stage work, or an oratorio
    Lbt,
    /// Licensee: A person or organization who is an original recipient of the right to print or publish
    Lse,
    /// Licensor: A person or organization who is a signer of the license, imprimatur, etc
    Lso,
    /// Lighting designer: A person or organization who designs the lighting scheme for a theatrical presentation, entertainment, motion picture, etc.
    Lgd,
    /// Lithographer: A person or organization who prepares the stone or plate for lithographic printing, including a graphic artist creating a design directly on the surface from which printing will be done.
    Ltg,
    /// Lyricist: An author of the words of a non-dramatic musical work (e.g. the text of a song), except for oratorios
    Lyr,
    /// Manufacture place: The place of manufacture (e.g., printing, duplicating, casting, etc.) of a resource in a published form
    Mfp,
    /// Manufacturer: A person or organization responsible for printing, duplicating, casting, etc. a resource
    Mfr,
    /// Marbler: The entity responsible for marbling paper, cloth, leather, etc. used in construction of a resource
    Mrb,
    /// Markup editor: A person or organization performing the coding of SGML, HTML, or XML markup of metadata, text, etc.
    Mrk,
    /// Medium: A person held to be a channel of communication between the earthly world and a world
    Med,
    /// Metadata contact: A person or organization primarily responsible for compiling and maintaining the original description of a metadata set (e.g., geospatial metadata set)
    Mdc,
    /// Metal-engraver: An engraver responsible for decorations, illustrations, letters, etc. cut on a metal surface for printing or decoration
    Mte,
    /// Minute taker: A person, family, or organization responsible for recording the minutes of a meeting
    Mtk,
    /// Moderator: A performer contributing to a resource by leading a program (often broadcast) where topics are discussed, usually with participation of experts in fields related to the discussion
    Mod,
    /// Monitor: A person or organization that supervises compliance with the contract and is responsible for the report and controls its distribution. Sometimes referred to as the grantee, or controlling agency
    Mon,
    /// Music copyist: A person who transcribes or copies musical notation
    Mcp,
    /// Musical director: A person who coordinates the activities of the composer, the sound editor, and sound mixers for a moving image production or for a musical or dramatic presentation or entertainment
    Msd,
    /// Musician: A person or organization who performs music or contributes to the musical content of a work when it is not possible or desirable to identify the function more precisely
    Mus,
    /// Narrator: A performer contributing to a resource by reading or speaking in order to give an account of an act, occurrence, course of events, etc
    Nrt,
    /// Onscreen presenter: A performer contributing to an expression of a work by appearing on screen in nonfiction moving image materials or introductions to fiction moving image materials to provide contextual or background information. Use when another term (e.g., narrator, host) is either not applicable or not desired
    Osp,
    /// Opponent: A person or organization responsible for opposing a thesis or dissertation
    Opn,
    /// Organizer: A person, family, or organization organizing the exhibit, event, conference, etc., which gave rise to a resource
    Orm,
    /// Originator: A person or organization performing the work, i.e., the name of a person or organization associated with the intellectual content of the work. This category does not include the publisher or personal affiliation, or sponsor except where it is also the corporate author
    Org,
    /// Other: A role that has no equivalent in the MARC list.
    Oth,
    /// Owner: A person, family, or organization that currently owns an item or collection, i.e.has legal possession of a resource
    Own,
    /// Panelist: A performer contributing to a resource by participating in a program (often broadcast) where topics are discussed, usually with participation of experts in fields related to the discussion
    Pan,
    /// Papermaker: A person or organization responsible for the production of paper, usually from wood, cloth, or other fibrous material
    Ppm,
    /// Patent applicant: A person or organization that applied for a patent
    Pta,
    /// Patent holder: A person or organization that was granted the patent referred to by the item
    Pth,
    /// Patron: A person or organization responsible for commissioning a work. Usually a patron uses his or her means or influence to support the work of artists, writers, etc. This includes those who commission and pay for individual works
    Pat,
    /// Performer: A person contributing to a resource by performing music, acting, dancing, speaking, etc., often in a musical or dramatic presentation, etc. If specific codes are used, [prf] is used for a person whose principal skill is not known or specified
    Prf,
    /// Permitting agency: An organization (usually a government agency) that issues permits under which work is accomplished
    Pma,
    /// Photographer: A person, family, or organization responsible for creating a photographic work
    Pht,
    /// Plaintiff: A person or organization who brings a suit in a civil proceeding
    Ptf,
    /// Plaintiff-appellant: A plaintiff who takes an appeal from one court or jurisdiction to another to reverse the judgment, usually in a legal proceeding
    Ptt,
    /// Plaintiff-appellee: A plaintiff against whom an appeal is taken from one court or jurisdiction to another to reverse the judgment, usually in a legal proceeding
    Pte,
    /// Platemaker: A person, family, or organization involved in manufacturing a manifestation by preparing plates used in the production of printed images and/or text
    Plt,
    /// Praeses: A person who is the faculty moderator of an academic disputation, normally proposing a thesis and participating in the ensuing disputation
    Pra,
    /// Presenter: A person or organization mentioned in an “X presents” credit for moving image materials and who is associated with production, finance, or distribution in some way. A vanity credit; in early years, normally the head of a studio
    Pre,
    /// Printer: A person, family, or organization involved in manufacturing a manifestation of printed text, notated music, etc., from type or plates, such as a book, newspaper, magazine, broadside, score, etc
    Prt,
    /// Printer of plates: A person or organization who prints illustrations from plates.
    Pop,
    /// Printmaker: A person or organization who makes a relief, intaglio, or planographic printing surface
    Prm,
    /// Process contact: A person or organization primarily responsible for performing or initiating a process, such as is done with the collection of metadata sets
    Prc,
    /// Producer: A person, family, or organization responsible for most of the business aspects of a production for screen, audio recording, television, webcast, etc. The producer is generally responsible for fund raising, managing the production, hiring key personnel, arranging for distributors, etc.
    Pro,
    /// Production company: An organization that is responsible for financial, technical, and organizational management of a production for stage, screen, audio recording, television, webcast, etc.
    Prn,
    /// Production designer: A person or organization responsible for designing the overall visual appearance of a moving image production
    Prs,
    /// Production manager: A person responsible for all technical and business matters in a production
    Pmn,
    /// Production personnel: A person or organization associated with the production (props, lighting, special effects, etc.) of a musical or dramatic presentation or entertainment
    Prd,
    /// Production place: The place of production (e.g., inscription, fabrication, construction, etc.) of a resource in an unpublished form
    Prp,
    /// Programmer: A person, family, or organization responsible for creating a computer program
    Prg,
    /// Project director: A person or organization with primary responsibility for all essential aspects of a project, has overall responsibility for managing projects, or provides overall direction to a project manager
    Pdr,
    /// Proofreader: A person who corrects printed matter. For manuscripts, use Corrector [crr]
    Pfr,
    /// Provider: A person or organization who produces, publishes, manufactures, or distributes a resource if specific codes are not desired (e.g. [mfr], [pbl])
    Prv,
    /// Publication place: The place where a resource is published
    Pup,
    /// Publisher: A person or organization responsible for publishing, releasing, or issuing a resource
    Pbl,
    /// Publishing director: A person or organization who presides over the elaboration of a collective work to ensure its coherence or continuity. This includes editors-in-chief, literary editors, editors of series, etc.
    Pbd,
    /// Puppeteer: A performer contributing to a resource by manipulating, controlling, or directing puppets or marionettes in a moving image production or a musical or dramatic presentation or entertainment
    Ppt,
    /// Radio director: A director responsible for the general management and supervision of a radio program
    Rdd,
    /// Radio producer: A producer responsible for most of the business aspects of a radio program
    Rpc,
    /// Recording engineer: A person contributing to a resource by supervising the technical aspects of a sound or video recording session
    Rce,
    /// Recordist: A person or organization who uses a recording device to capture sounds and/or video during a recording session, including field recordings of natural sounds, folkloric events, music, etc.
    Rcd,
    /// Redaktor: A person or organization who writes or develops the framework for an item without being intellectually responsible for its content
    Red,
    /// Renderer: A person or organization who prepares drawings of architectural designs (i.e., renderings) in accurate, representational perspective to show what the project will look like when completed
    Ren,
    /// Reporter: A person or organization who writes or presents reports of news or current events on air or in print
    Rpt,
    /// Repository: An organization that hosts data or material culture objects and provides services to promote long term, consistent and shared use of those data or objects
    Rps,
    /// Research team head: A person who directed or managed a research project
    Rth,
    /// Research team member: A person who participated in a research project but whose role did not involve direction or management of it
    Rtm,
    /// Researcher: A person or organization responsible for performing research
    Res,
    /// Respondent: A person or organization who makes an answer to the courts pursuant to an application for redress (usually in an equity proceeding) or a candidate for a degree who defends or opposes a thesis provided by the praeses in an academic disputation
    Rsp,
    /// Respondent-appellant: A respondent who takes an appeal from one court or jurisdiction to another to reverse the judgment, usually in an equity proceeding
    Rst,
    /// Respondent-appellee: A respondent against whom an appeal is taken from one court or jurisdiction to another to reverse the judgment, usually in an equity proceeding
    Rse,
    /// Responsible party: A person or organization legally responsible for the content of the published material
    Rpy,
    /// Restager: A person or organization, other than the original choreographer or director, responsible for restaging a choreographic or dramatic work and who contributes minimal new content
    Rsg,
    /// Restorationist: A person, family, or organization responsible for the set of technical, editorial, and intellectual procedures aimed at compensating for the degradation of an item by bringing it back to a state as close as possible to its original condition
    Rsr,
    /// Reviewer: A person or organization responsible for the review of a book, motion picture, performance, etc.
    Rev,
    /// Rubricator: A person or organization responsible for parts of a work, often headings or opening parts of a manuscript, that appear in a distinctive color, usually red
    Rbr,
    /// Scenarist: A person or organization who is the author of a motion picture screenplay, generally the person who wrote the scenarios for a motion picture during the silent era
    Sce,
    /// Scientific advisor: A person or organization who brings scientific, pedagogical, or historical competence to the conception and realization on a work, particularly in the case of audio-visual items
    Sad,
    /// Screenwriter: An author of a screenplay, script, or scene
    Aus,
    /// Scribe: A person who is an amanuensis and for a writer of manuscripts proper. For a person who makes pen-facsimiles, use Facsimilist [fac]
    Scr,
    /// Sculptor: An artist responsible for creating a three-dimensional work by modeling, carving, or similar technique
    Scl,
    /// Second party: A person or organization who is identified as the party of the second part. In the case of transfer of right, this is the assignee, transferee, licensee, grantee, etc. Multiple parties can be named jointly as the second party
    Spy,
    /// Secretary: A person or organization who is a recorder, redactor, or other person responsible for expressing the views of a organization
    Sec,
    /// Seller: A former owner of an item who sold that item to another owner
    Sll,
    /// Set designer: A person who translates the rough sketches of the art director into actual architectural structures for a theatrical presentation, entertainment, motion picture, etc. Set designers draw the detailed guides and specifications for building the set
    Std,
    /// Setting: An entity in which the activity or plot of a work takes place, e.g. a geographic place, a time period, a building, an event
    Stg,
    /// Signer: A person whose signature appears without a presentation or other statement indicative of provenance. When there is a presentation statement, use Inscriber [ins].
    Sgn,
    /// Singer: A performer contributing to a resource by using his/her/their voice, with or without instrumental accompaniment, to produce music. A singer's performance may or may not include actual words
    Sng,
    /// Sound designer: A person who produces and reproduces the sound score (both live and recorded), the installation of microphones, the setting of sound levels, and the coordination of sources of sound for a production
    Sds,
    /// Speaker: A performer contributing to a resource by speaking words, such as a lecture, speech, etc.  
    Spk,
    /// Sponsor: A person, family, or organization sponsoring some aspect of a resource, e.g., funding research, sponsoring an event
    Spn,
    /// Stage director: A person or organization contributing to a stage resource through the overall management and supervision of a performance
    Sgd,
    /// Stage manager: A person who is in charge of everything that occurs on a performance stage, and who acts as chief of all crews and assistant to a director during rehearsals
    Stm,
    /// Standards body: An organization responsible for the development or enforcement of a standard
    Stn,
    /// Stereotyper: A person or organization who creates a new plate for printing by molding or copying another printing surface
    Str,
    /// Storyteller: A performer contributing to a resource by relaying a creator's original story with dramatic or theatrical interpretation
    Stl,
    /// Supporting host: A person or organization that supports (by allocating facilities, staff, or other resources) a project, program, meeting, event, data objects, material culture objects, or other entities capable of support
    Sht,
    /// Surveyor: A person, family, or organization contributing to a cartographic resource by providing measurements or dimensional relationships for the geographic area represented
    Srv,
    /// Teacher: A performer contributing to a resource by giving instruction or providing a demonstration
    Tch,
    /// Technical director: A person who is ultimately in charge of scenery, props, lights and sound for a production
    Tcd,
    /// Television director: A director responsible for the general management and supervision of a television program
    Tld,
    /// Television producer: A producer responsible for most of the business aspects of a television program
    Tlp,
    /// Thesis advisor: A person under whose supervision a degree candidate develops and presents a thesis, mémoire, or text of a dissertation
    Ths,
    /// Transcriber: A person, family, or organization contributing to a resource by changing it from one system of notation to another. For a work transcribed for a different instrument or performing group, see Arranger [arr]. For makers of pen-facsimiles, use Facsimilist [fac]
    Trc,
    /// Translator: A person or organization who renders a text from one language into another, or from an older form of a language into the modern form
    Trl,
    /// Type designer: A person or organization who designs the type face used in a particular item
    Tyd,
    /// Typographer: A person or organization primarily responsible for choice and arrangement of type used in an item. If the typographer is also responsible for other aspects of the graphic design of a book (e.g., Book designer [bkd]), codes for both functions may be needed
    Tyg,
    /// University place: A place where a university that is associated with a resource is located, for example, a university where an academic dissertation or thesis was presented
    Uvp,
    /// Videographer: A person in charge of a video production, e.g. the video recording of a stage production as opposed to a commercial motion picture. The videographer may be the camera operator or may supervise one or more camera operators. Do not confuse with cinematographer
    Vdg,
    /// Voice actor: An actor contributing to a resource by providing the voice for characters in radio and audio productions and for animated characters in moving image works, as well as by providing voice overs in radio and television commercials, dubbed resources, etc.
    Vac,
    /// Witness: Use for a person who verifies the truthfulness of an event or action.
    Wit,
    /// Wood engraver: A person or organization who makes prints by cutting the image in relief on the end-grain of a wood block
    Wde,
    /// Woodcutter: A person or organization who makes prints by cutting the image in relief on the plank side of a wood block
    Wdc,
    /// Writer of accompanying material: A person or organization who writes significant material which accompanies a sound recording or other audiovisual material
    Wam,
    /// Writer of added commentary: A person, family, or organization contributing to an expression of a work by providing an interpretation or critical explanation of the original work
    Wac,
    /// Writer of added lyrics: A writer of words added to an expression of a musical work. For lyric writing in collaboration with a composer to form an original work, see lyricist
    Wal,
    /// Writer of added text: A person, family, or organization contributing to a non-textual resource by providing text for the non-textual work (e.g., writing captions for photographs, descriptions of maps).
    Wat,
    /// Writer of introduction: A person, family, or organization contributing to a resource by providing an introduction to the original work
    Win,
    /// Writer of preface: A person, family, or organization contributing to a resource by providing a preface to the original work
    Wpr,
    /// Writer of supplementary textual content: A person, family, or organization contributing to a resource by providing supplementary textual content (e.g., an introduction, a preface) to the original work
    Wst,
}

impl FromStr for MarcRelator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // the human readable names here are the ones pandoc uses
        match s {
            "abridger" | "abr" => Ok(MarcRelator::Abr),
            "actor" | "act" => Ok(MarcRelator::Act),
            "adapter" | "adp" => Ok(MarcRelator::Adp),
            "addressee" | "rcp" => Ok(MarcRelator::Rcp),
            "analyst" | "anl" => Ok(MarcRelator::Anl),
            "animator" | "anm" => Ok(MarcRelator::Anm),
            "annotator" | "ann" => Ok(MarcRelator::Ann),
            "appellant" | "apl" => Ok(MarcRelator::Apl),
            "appellee" | "ape" => Ok(MarcRelator::Ape),
            "applicant" | "app" => Ok(MarcRelator::App),
            "architect" | "arc" => Ok(MarcRelator::Arc),
            "arranger" | "arr" => Ok(MarcRelator::Arr),
            "art copyist" | "acp" => Ok(MarcRelator::Acp),
            "art director" | "adi" => Ok(MarcRelator::Adi),
            "artist" | "art" => Ok(MarcRelator::Art),
            "artistic director" | "ard" => Ok(MarcRelator::Ard),
            "assignee" | "asg" => Ok(MarcRelator::Asg),
            "associated name" | "asn" => Ok(MarcRelator::Asn),
            "attributed name" | "att" => Ok(MarcRelator::Att),
            "auctioneer" | "auc" => Ok(MarcRelator::Auc),
            "author" | "aut" => Ok(MarcRelator::Aut),
            "author in quotations or text abstracts" | "aqt" => Ok(MarcRelator::Aqt),
            "author of afterword, colophon, etc." | "aft" => Ok(MarcRelator::Aft),
            "author of dialog" | "aud" => Ok(MarcRelator::Aud),
            "author of introduction, etc." | "aui" => Ok(MarcRelator::Aui),
            "autographer" | "ato" => Ok(MarcRelator::Ato),
            "bibliographic antecedent" | "ant" => Ok(MarcRelator::Ant),
            "binder" | "bnd" => Ok(MarcRelator::Bnd),
            "binding designer" | "bdd" => Ok(MarcRelator::Bdd),
            "blurb writer" | "blw" => Ok(MarcRelator::Blw),
            "book designer" | "bkd" => Ok(MarcRelator::Bkd),
            "book producer" | "bkp" => Ok(MarcRelator::Bkp),
            "bookjacket designer" | "bjd" => Ok(MarcRelator::Bjd),
            "bookplate designer" | "bpd" => Ok(MarcRelator::Bpd),
            "bookseller" | "bsl" => Ok(MarcRelator::Bsl),
            "braille embosser" | "brl" => Ok(MarcRelator::Brl),
            "broadcaster" | "brd" => Ok(MarcRelator::Brd),
            "calligrapher" | "cll" => Ok(MarcRelator::Cll),
            "cartographer" | "ctg" => Ok(MarcRelator::Ctg),
            "caster" | "cas" => Ok(MarcRelator::Cas),
            "censor" | "cns" => Ok(MarcRelator::Cns),
            "choreographer" | "chr" => Ok(MarcRelator::Chr),
            "cinematographer" | "cng" => Ok(MarcRelator::Cng),
            "client" | "cli" => Ok(MarcRelator::Cli),
            "collection registrar" | "cor" => Ok(MarcRelator::Cor),
            "collector" | "col" => Ok(MarcRelator::Col),
            "collotyper" | "clt" => Ok(MarcRelator::Clt),
            "colorist" | "clr" => Ok(MarcRelator::Clr),
            "commentator" | "cmm" => Ok(MarcRelator::Cmm),
            "commentator for written text" | "cwt" => Ok(MarcRelator::Cwt),
            "compiler" | "com" => Ok(MarcRelator::Com),
            "complainant" | "cpl" => Ok(MarcRelator::Cpl),
            "complainant-appellant" | "cpt" => Ok(MarcRelator::Cpt),
            "complainant-appellee" | "cpe" => Ok(MarcRelator::Cpe),
            "composer" | "cmp" => Ok(MarcRelator::Cmp),
            "compositor" | "cmt" => Ok(MarcRelator::Cmt),
            "conceptor" | "ccp" => Ok(MarcRelator::Ccp),
            "conductor" | "cnd" => Ok(MarcRelator::Cnd),
            "conservator" | "con" => Ok(MarcRelator::Con),
            "consultant" | "csl" => Ok(MarcRelator::Csl),
            "consultant to a project" | "csp" => Ok(MarcRelator::Csp),
            "contestant" | "cos" => Ok(MarcRelator::Cos),
            "contestant-appellant" | "cot" => Ok(MarcRelator::Cot),
            "contestant-appellee" | "coe" => Ok(MarcRelator::Coe),
            "contestee" | "cts" => Ok(MarcRelator::Cts),
            "contestee-appellant" | "ctt" => Ok(MarcRelator::Ctt),
            "contestee-appellee" | "cte" => Ok(MarcRelator::Cte),
            "contractor" | "ctr" => Ok(MarcRelator::Ctr),
            "contributor" | "ctb" => Ok(MarcRelator::Ctb),
            "copyright claimant" | "cpc" => Ok(MarcRelator::Cpc),
            "copyright holder" | "cph" => Ok(MarcRelator::Cph),
            "corrector" | "crr" => Ok(MarcRelator::Crr),
            "correspondent" | "crp" => Ok(MarcRelator::Crp),
            "costume designer" | "cst" => Ok(MarcRelator::Cst),
            "court governed" | "cou" => Ok(MarcRelator::Cou),
            "court reporter" | "crt" => Ok(MarcRelator::Crt),
            "cover designer" | "cov" => Ok(MarcRelator::Cov),
            "creator" | "cre" => Ok(MarcRelator::Cre),
            "curator" | "cur" => Ok(MarcRelator::Cur),
            "dancer" | "dnc" => Ok(MarcRelator::Dnc),
            "data contributor" | "dtc" => Ok(MarcRelator::Dtc),
            "data manager" | "dtm" => Ok(MarcRelator::Dtm),
            "dedicatee" | "dte" => Ok(MarcRelator::Dte),
            "dedicator" | "dto" => Ok(MarcRelator::Dto),
            "defendant" | "dfd" => Ok(MarcRelator::Dfd),
            "defendant-appellant" | "dft" => Ok(MarcRelator::Dft),
            "defendant-appellee" | "dfe" => Ok(MarcRelator::Dfe),
            "degree granting institution" | "dgg" => Ok(MarcRelator::Dgg),
            "delineator" | "dln" => Ok(MarcRelator::Dln),
            "depicted" | "dpc" => Ok(MarcRelator::Dpc),
            "depositor" | "dpt" => Ok(MarcRelator::Dpt),
            "designer" | "dsr" => Ok(MarcRelator::Dsr),
            "director" | "drt" => Ok(MarcRelator::Drt),
            "dissertant" | "dis" => Ok(MarcRelator::Dis),
            "distribution place" | "dbp" => Ok(MarcRelator::Dbp),
            "distributor" | "dst" => Ok(MarcRelator::Dst),
            "donor" | "dnr" => Ok(MarcRelator::Dnr),
            "draftsman" | "drm" => Ok(MarcRelator::Drm),
            "dubious author" | "dub" => Ok(MarcRelator::Dub),
            "editor" | "edt" => Ok(MarcRelator::Edt),
            "editor of compilation" | "edc" => Ok(MarcRelator::Edc),
            "editor of moving image work" | "edm" => Ok(MarcRelator::Edm),
            "electrician" | "elg" => Ok(MarcRelator::Elg),
            "electrotyper" | "elt" => Ok(MarcRelator::Elt),
            "enacting jurisdiction" | "enj" => Ok(MarcRelator::Enj),
            "engineer" | "eng" => Ok(MarcRelator::Eng),
            "engraver" | "egr" => Ok(MarcRelator::Egr),
            "etcher" | "etr" => Ok(MarcRelator::Etr),
            "event place" | "evp" => Ok(MarcRelator::Evp),
            "expert" | "exp" => Ok(MarcRelator::Exp),
            "facsimilist" | "fac" => Ok(MarcRelator::Fac),
            "field director" | "fld" => Ok(MarcRelator::Fld),
            "film director" | "fmd" => Ok(MarcRelator::Fmd),
            "film distributor" | "fds" => Ok(MarcRelator::Fds),
            "film editor" | "flm" => Ok(MarcRelator::Flm),
            "film producer" | "fmp" => Ok(MarcRelator::Fmp),
            "filmmaker" | "fmk" => Ok(MarcRelator::Fmk),
            "first party" | "fpy" => Ok(MarcRelator::Fpy),
            "forger" | "frg" => Ok(MarcRelator::Frg),
            "former owner" | "fmo" => Ok(MarcRelator::Fmo),
            "funder" | "fnd" => Ok(MarcRelator::Fnd),
            "geographic information specialist" | "gis" => Ok(MarcRelator::Gis),
            "honoree" | "hnr" => Ok(MarcRelator::Hnr),
            "host" | "hst" => Ok(MarcRelator::Hst),
            "host institution" | "his" => Ok(MarcRelator::His),
            "illuminator" | "ilu" => Ok(MarcRelator::Ilu),
            "illustrator" | "ill" => Ok(MarcRelator::Ill),
            "inscriber" | "ins" => Ok(MarcRelator::Ins),
            "instrumentalist" | "itr" => Ok(MarcRelator::Itr),
            "interviewee" | "ive" => Ok(MarcRelator::Ive),
            "interviewer" | "ivr" => Ok(MarcRelator::Ivr),
            "inventor" | "inv" => Ok(MarcRelator::Inv),
            "issuing body" | "isb" => Ok(MarcRelator::Isb),
            "judge" | "jud" => Ok(MarcRelator::Jud),
            "jurisdiction governed" | "jug" => Ok(MarcRelator::Jug),
            "laboratory" | "lbr" => Ok(MarcRelator::Lbr),
            "laboratory director" | "ldr" => Ok(MarcRelator::Ldr),
            "landscape architect" | "lsa" => Ok(MarcRelator::Lsa),
            "lead" | "led" => Ok(MarcRelator::Led),
            "lender" | "len" => Ok(MarcRelator::Len),
            "libelant" | "lil" => Ok(MarcRelator::Lil),
            "libelant-appellant" | "lit" => Ok(MarcRelator::Lit),
            "libelant-appellee" | "lie" => Ok(MarcRelator::Lie),
            "libelee" | "lel" => Ok(MarcRelator::Lel),
            "libelee-appellant" | "let" => Ok(MarcRelator::Let),
            "libelee-appellee" | "lee" => Ok(MarcRelator::Lee),
            "librettist" | "lbt" => Ok(MarcRelator::Lbt),
            "licensee" | "lse" => Ok(MarcRelator::Lse),
            "licensor" | "lso" => Ok(MarcRelator::Lso),
            "lighting designer" | "lgd" => Ok(MarcRelator::Lgd),
            "lithographer" | "ltg" => Ok(MarcRelator::Ltg),
            "lyricist" | "lyr" => Ok(MarcRelator::Lyr),
            "manufacture place" | "mfp" => Ok(MarcRelator::Mfp),
            "manufacturer" | "mfr" => Ok(MarcRelator::Mfr),
            "marbler" | "mrb" => Ok(MarcRelator::Mrb),
            "markup editor" | "mrk" => Ok(MarcRelator::Mrk),
            "metadata contact" | "mdc" => Ok(MarcRelator::Mdc),
            "metal-engraver" | "mte" => Ok(MarcRelator::Mte),
            "moderator" | "mod" => Ok(MarcRelator::Mod),
            "monitor" | "mon" => Ok(MarcRelator::Mon),
            "music copyist" | "mcp" => Ok(MarcRelator::Mcp),
            "musical director" | "msd" => Ok(MarcRelator::Msd),
            "musician" | "mus" => Ok(MarcRelator::Mus),
            "narrator" | "nrt" => Ok(MarcRelator::Nrt),
            "onscreen presenter" | "osp" => Ok(MarcRelator::Osp),
            "opponent" | "opn" => Ok(MarcRelator::Opn),
            "organizer of meeting" | "orm" => Ok(MarcRelator::Orm),
            "originator" | "org" => Ok(MarcRelator::Org),
            "other" | "oth" => Ok(MarcRelator::Oth),
            "owner" | "own" => Ok(MarcRelator::Own),
            "panelist" | "pan" => Ok(MarcRelator::Pan),
            "papermaker" | "ppm" => Ok(MarcRelator::Ppm),
            "patent applicant" | "pta" => Ok(MarcRelator::Pta),
            "patent holder" | "pth" => Ok(MarcRelator::Pth),
            "patron" | "pat" => Ok(MarcRelator::Pat),
            "performer" | "prf" => Ok(MarcRelator::Prf),
            "permitting agency" | "pma" => Ok(MarcRelator::Pma),
            "photographer" | "pht" => Ok(MarcRelator::Pht),
            "plaintiff" | "ptf" => Ok(MarcRelator::Ptf),
            "plaintiff-appellant" | "ptt" => Ok(MarcRelator::Ptt),
            "plaintiff-appellee" | "pte" => Ok(MarcRelator::Pte),
            "platemaker" | "plt" => Ok(MarcRelator::Plt),
            "praeses" | "pra" => Ok(MarcRelator::Pra),
            "presenter" | "pre" => Ok(MarcRelator::Pre),
            "printer" | "prt" => Ok(MarcRelator::Prt),
            "printer of plates" | "pop" => Ok(MarcRelator::Pop),
            "printmaker" | "prm" => Ok(MarcRelator::Prm),
            "process contact" | "prc" => Ok(MarcRelator::Prc),
            "producer" | "pro" => Ok(MarcRelator::Pro),
            "production company" | "prn" => Ok(MarcRelator::Prn),
            "production designer" | "prs" => Ok(MarcRelator::Prs),
            "production manager" | "pmn" => Ok(MarcRelator::Pmn),
            "production personnel" | "prd" => Ok(MarcRelator::Prd),
            "production place" | "prp" => Ok(MarcRelator::Prp),
            "programmer" | "prg" => Ok(MarcRelator::Prg),
            "project director" | "pdr" => Ok(MarcRelator::Pdr),
            "proofreader" | "pfr" => Ok(MarcRelator::Pfr),
            "provider" | "prv" => Ok(MarcRelator::Prv),
            "publication place" | "pup" => Ok(MarcRelator::Pup),
            "publisher" | "pbl" => Ok(MarcRelator::Pbl),
            "publishing director" | "pbd" => Ok(MarcRelator::Pbd),
            "puppeteer" | "ppt" => Ok(MarcRelator::Ppt),
            "radio director" | "rdd" => Ok(MarcRelator::Rdd),
            "radio producer" | "rpc" => Ok(MarcRelator::Rpc),
            "recording engineer" | "rce" => Ok(MarcRelator::Rce),
            "recordist" | "rcd" => Ok(MarcRelator::Rcd),
            "redaktor" | "red" => Ok(MarcRelator::Red),
            "renderer" | "ren" => Ok(MarcRelator::Ren),
            "reporter" | "rpt" => Ok(MarcRelator::Rpt),
            "repository" | "rps" => Ok(MarcRelator::Rps),
            "research team head" | "rth" => Ok(MarcRelator::Rth),
            "research team member" | "rtm" => Ok(MarcRelator::Rtm),
            "researcher" | "res" => Ok(MarcRelator::Res),
            "respondent" | "rsp" => Ok(MarcRelator::Rsp),
            "respondent-appellant" | "rst" => Ok(MarcRelator::Rst),
            "respondent-appellee" | "rse" => Ok(MarcRelator::Rse),
            "responsible party" | "rpy" => Ok(MarcRelator::Rpy),
            "restager" | "rsg" => Ok(MarcRelator::Rsg),
            "restorationist" | "rsr" => Ok(MarcRelator::Rsr),
            "reviewer" | "rev" => Ok(MarcRelator::Rev),
            "rubricator" | "rbr" => Ok(MarcRelator::Rbr),
            "scenarist" | "sce" => Ok(MarcRelator::Sce),
            "scientific advisor" | "sad" => Ok(MarcRelator::Sad),
            "screenwriter" | "aus" => Ok(MarcRelator::Aus),
            "scribe" | "scr" => Ok(MarcRelator::Scr),
            "sculptor" | "scl" => Ok(MarcRelator::Scl),
            "second party" | "spy" => Ok(MarcRelator::Spy),
            "secretary" | "sec" => Ok(MarcRelator::Sec),
            "seller" | "sll" => Ok(MarcRelator::Sll),
            "set designer" | "std" => Ok(MarcRelator::Std),
            "setting" | "stg" => Ok(MarcRelator::Stg),
            "signer" | "sgn" => Ok(MarcRelator::Sgn),
            "singer" | "sng" => Ok(MarcRelator::Sng),
            "sound designer" | "sds" => Ok(MarcRelator::Sds),
            "speaker" | "spk" => Ok(MarcRelator::Spk),
            "sponsor" | "spn" => Ok(MarcRelator::Spn),
            "stage director" | "sgd" => Ok(MarcRelator::Sgd),
            "stage manager" | "stm" => Ok(MarcRelator::Stm),
            "standards body" | "stn" => Ok(MarcRelator::Stn),
            "stereotyper" | "str" => Ok(MarcRelator::Str),
            "storyteller" | "stl" => Ok(MarcRelator::Stl),
            "supporting host" | "sht" => Ok(MarcRelator::Sht),
            "surveyor" | "srv" => Ok(MarcRelator::Srv),
            "teacher" | "tch" => Ok(MarcRelator::Tch),
            "technical director" | "tcd" => Ok(MarcRelator::Tcd),
            "television director" | "tld" => Ok(MarcRelator::Tld),
            "television producer" | "tlp" => Ok(MarcRelator::Tlp),
            "thesis advisor" | "ths" => Ok(MarcRelator::Ths),
            "transcriber" | "trc" => Ok(MarcRelator::Trc),
            "translator" | "trl" => Ok(MarcRelator::Trl),
            "type designer" | "tyd" => Ok(MarcRelator::Tyd),
            "typographer" | "tyg" => Ok(MarcRelator::Tyg),
            "university place" | "uvp" => Ok(MarcRelator::Uvp),
            "videographer" | "vdg" => Ok(MarcRelator::Vdg),
            "witness" | "wit" => Ok(MarcRelator::Wit),
            "wood engraver" | "wde" => Ok(MarcRelator::Wde),
            "woodcutter" | "wdc" => Ok(MarcRelator::Wdc),
            "writer of accompanying material" | "wam" => Ok(MarcRelator::Wam),
            "writer of added commentary" | "wac" => Ok(MarcRelator::Wac),
            "writer of added lyrics" | "wal" => Ok(MarcRelator::Wal),
            "writer of added text" | "wat" => Ok(MarcRelator::Wat),
            _ => Err(()),
        }
    }
}

/// The default title types from the epub standard (<https://www.w3.org/publishing/epub/epub-packages.html#sec-title-type>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EpubTitleType {
    Main,
    Subtitle,
    Short,
    Collection,
    Edition,
    Expanded,
}

impl FromStr for EpubTitleType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "main" => Ok(EpubTitleType::Main),
            "subtitle" => Ok(EpubTitleType::Subtitle),
            "short" => Ok(EpubTitleType::Short),
            "collection" => Ok(EpubTitleType::Collection),
            "edition" => Ok(EpubTitleType::Edition),
            "expanded" => Ok(EpubTitleType::Expanded),
            _ => Err(()),
        }
    }
}

/// Dublin core terms (<https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#section-2>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DublinCoreTerm {
    Abstract,
    Accessrights,
    Accrualmethod,
    Accrualperiodicity,
    Accrualpolicy,
    Alternative,
    Audience,
    Available,
    Bibliographiccitation,
    Conformsto,
    Contributor,
    Coverage,
    Created,
    Creator,
    Date,
    Dateaccepted,
    Datecopyrighted,
    Datesubmitted,
    Description,
    Educationlevel,
    Extent,
    Format,
    Hasformat,
    Haspart,
    Hasversion,
    Identifier,
    Instructionalmethod,
    Isformatof,
    Ispartof,
    Isreferencedby,
    Isreplacedby,
    Isrequiredby,
    Isversionof,
    Issued,
    Language,
    License,
    Mediator,
    Medium,
    Modified,
    Provenance,
    Publisher,
    References,
    Relation,
    Replaces,
    Requires,
    Rights,
    Rightsholder,
    Source,
    Spatial,
    Subject,
    Tableofcontents,
    Temporal,
    Title,
    Type,
    Valid,
}

/// Dublin Core elements (<https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#section-3>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DublinCoreElement {
    Contributor,
    Coverage,
    Creator,
    Date,
    Description,
    Format,
    Identifier,
    Language,
    Publisher,
    Relation,
    Rights,
    Source,
    Subject,
    Title,
    Type,
}

impl DublinCoreElement {
    pub fn as_tagname(&self) -> String {
        format!("dc:{:?}", self).to_lowercase()
    }
}

/// Onix codelist 5 (<https://ns.editeur.org/onix/en/5>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum OnixProductIdentifier {
    /// Proprietary: For example, a publisher’s or wholesaler’s product number or SKU. Note that <IDTypeName> is required with proprietary identifiers
    I01,
    /// ISBN-10: International Standard Book Number, pre-2007 (10 digits, or 9 digits plus X, without spaces or hyphens) – now DEPRECATED in ONIX for Books, except where providing historical information for compatibility with legacy systems. It should only be used in relation to products published before 2007 – when ISBN-13 superseded it – and should never be used as the ONLY identifier (it should always be accompanied by the correct GTIN-13 / ISBN-13)
    I02,
    /// GTIN-13: GS1 Global Trade Item Number, formerly known as EAN article number (13 digits, without spaces or hyphens)
    I03,
    /// UPC: UPC product number (12 digits, without spaces or hyphens)
    I04,
    /// ISMN-10: International Standard Music Number, pre-2008 (M plus nine digits, without spaces or hyphens) – now DEPRECATED in ONIX for Books, except where providing historical information for compatibility with legacy systems. It should only be used in relation to products published before 2008 – when ISMN-13 superseded it – and should never be used as the ONLY identifier (it should always be accompanied by the correct GTIN-12 / ISMN-13)
    I05,
    /// DOI: Digital Object Identifier (variable length and character set beginning ‘10.’, and without https://doi.org/ or the older http://dx.doi.org/)
    I06,
    /// LCCN: Library of Congress Control Number in normalized form (up to 12 characters, alphanumeric)
    I13,
    /// GTIN-14: GS1 Global Trade Item Number (14 digits, without spaces or hyphens)
    I14,
    /// ISBN-13: International Standard Book Number, from 2007 (13 digits starting 978 or 9791–9799, without spaces or hypens)
    I15,
    /// Legal deposit number: The number assigned to a publication as part of a national legal deposit process
    I17,
    /// URN: Uniform Resource Name: note that in trade applications an ISBN must be sent as a GTIN-13 and, where required, as an ISBN-13 – it should not be sent as a URN
    I22,
    /// OCLC number: A unique number assigned to a bibliographic item by OCLC
    I23,
    /// Co-publisher’s ISBN-13: An ISBN-13 assigned by a co-publisher. The ‘main’ ISBN sent with <ProductIDType> codes 03 and/or 15 should always be the ISBN that is used for ordering from the supplier identified in <SupplyDetail>. However, ISBN rules allow a co-published title to carry more than one ISBN. The co-publisher should be identified in an instance of the <Publisher> composite, with the applicable <PublishingRole> code
    I24,
    /// ISMN-13: International Standard Music Number, from 2008 (13-digit number starting 9790, without spaces or hyphens)
    I25,
    /// ISBN-A: Actionable ISBN, in fact a special DOI incorporating the ISBN-13 within the DOI syntax. Begins ‘10.978.’ or ‘10.979.’ and includes a / character between the registrant element (publisher prefix) and publication element of the ISBN, eg 10.978.000/1234567. Note the ISBN-A should always be accompanied by the ISBN itself, using <ProductIDType> codes 03 and/or 15
    I26,
    /// JP e-code: E-publication identifier controlled by JPOIID’s Committee for Research and Management of Electronic Publishing Codes
    I27,
    /// OLCC number: Unique number assigned by the Chinese Online Library Cataloging Center (see http://olcc.nlc.gov.cn)
    I28,
    /// JP Magazine ID: Japanese magazine identifier, similar in scope to ISSN but identifying a specific issue of a serial publication. Five digits to identify the periodical, plus a hyphen and two digits to identify the issue
    I29,
    /// UPC12+5: Used only with comic books and other products which use the UPC extension to identify individual issues or products. Do not use where the UPC12 itself identifies the specific product, irrespective of any 5-digit extension – use code 04 instead
    I30,
    /// BNF Control number: Numéro de la notice bibliographique BNF
    I31,
    /// ARK: Archival Resource Key, as a URL (including the address of the ARK resolver provided by eg a national library)
    I35,
}

/// Onix codelist 15 (<https://ns.editeur.org/onix/en/15>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum OnixTitleCode {
    /// Undefined
    T00,
    /// Distinctive title (book); Cover title (serial); Title on item (serial content item or reviewed resource)    The full text of the distinctive title of the item, without abbreviation or abridgement. For books, where the title alone is not distinctive, elements may be taken from a set or series title and part number etc to create a distinctive title. Where the item is an omnibus edition containing two or more works by the same author, and there is no separate combined title, a distinctive title may be constructed by concatenating the individual titles, with suitable punctuation, as in ‘Pride and prejudice / Sense and sensibility / Northanger Abbey’
    T01,
    /// ISSN key title of serial: Serials only
    T02,
    /// Title in original language: Where the subject of the ONIX record is a translated item
    T03,
    /// Title acronym or initialism: For serials: an acronym or initialism of Title Type1, eg ‘JAMA’, ‘JACM’
    T04,
    /// Abbreviated title: An abbreviated form of Title Type1
    T05,
    /// Title in other language: A translation of Title Type1 into another language
    T06,
    /// Thematic title of journal issue: Serials only: when a journal issue is explicitly devoted to a specified topic
    T07,
    /// Former title: Books or serials: when an item was previously published under another title
    T08,
    /// Distributor’s title: For books: the title carried in a book distributor’s title file: frequently incomplete, and may include elements not properly part of the title
    T10,
    /// Alternative title on cover: An alternative title that appears on the cover of a book
    T11,
    /// Alternative title on back: An alternative title that appears on the back of a book
    T12,
    /// Expanded title: An expanded form of the title, eg the title of a school text book with grade and type and other details added to make the title meaningful, where otherwise it would comprise only the curriculum subject. This title type is required for submissions to the Spanish ISBN Agency
    T13,
    /// Alternative title: An alternative title that the book is widely known by, whether it appears on the book or not
    T14,
}

/// Onix codelist 17 (<https://ns.editeur.org/onix/en/17>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum OnixContributorCode {
    /// By (author): Author of a textual work
    A01,
    /// With: With or as told to: ‘ghost’ or secondary author of a literary work (for clarity, should not be used for true ‘ghost’ authors who are not credited on the book and whose existence is secret)
    A02,
    /// Screenplay by: Writer of screenplay or script (film or video)
    A03,
    /// Libretto by: Writer of libretto (opera): see also A31
    A04,
    /// Lyrics by: Author of lyrics (song): see also A31
    A05,
    /// By (composer): Composer of music
    A06,
    /// By (artist): Visual artist when named as the primary creator of, eg, a book of reproductions of artworks
    A07,
    /// By (photographer): Photographer when named as the primary creator of, eg, a book of photographs
    A08,
    /// Created by
    A09,
    /// From an idea by
    A10,
    /// Designed by
    A11,
    /// Illustrated by: Artist when named as the creator of artwork which illustrates a text, or the originator (sometimes ‘penciller’ for collaborative art) of the artwork of a graphic novel or comic book
    A12,
    /// Photographs by: Photographer when named as the creator of photographs which illustrate a text
    A13,
    /// Text by: Author of text which accompanies art reproductions or photographs, or which is part of a graphic novel or comic book
    A14,
    /// Preface by: Author of preface
    A15,
    /// Prologue by: Author of prologue
    A16,
    /// Summary by: Author of summary
    A17,
    /// Supplement by: Author of supplement
    A18,
    /// Afterword by: Author of afterword
    A19,
    /// Notes by: Author of notes or annotations: see also A29
    A20,
    /// Commentaries by: Author of commentaries on the main text
    A21,
    /// Epilogue by: Author of epilogue
    A22,
    /// Foreword by: Author of foreword
    A23,
    /// Introduction by: Author of introduction: see also A29
    A24,
    /// Footnotes by: Author/compiler of footnotes
    A25,
    /// Memoir by: Author of memoir accompanying main text
    A26,
    /// Experiments by: Person who carried out experiments reported in the text
    A27,
    /// Introduction and notes by: Author of introduction and notes: see also A20 and A24
    A29,
    /// Software written by: Writer of computer programs ancillary to the text
    A30,
    /// Book and lyrics by: Author of the textual content of a musical drama: see also A04 and A05
    A31,
    /// Contributions by: Author of additional contributions to the text
    A32,
    /// Appendix by: Author of appendix
    A33,
    /// Index by: Compiler of index
    A34,
    /// Drawings by
    A35,
    /// Cover design or artwork by: Use also for the cover artist of a graphic novel or comic book if named separately
    A36,
    /// Preliminary work by: Responsible for preliminary work on which the work is based
    A37,
    /// Original author: Author of the first edition (usually of a standard work) who is not an author of the current edition
    A38,
    /// Maps by: Maps drawn or otherwise contributed by
    A39,
    /// Inked or colored by: Use for secondary creators when separate persons are named as having respectively drawn and inked/colored/finished artwork, eg for a graphic novel or comic book. Use with A12 for ‘drawn by’. Use A40 for ‘finished by’, but prefer more specific codes A46 to A48 instead of A40 unless the more specific secondary roles are inappropriate, unclear or unavailable
    A40,
    /// Paper engineering by: Designer or paper engineer of die-cuts, press-outs or of pop-ups in a pop-up book, who may be different from the illustrator
    A41,
    /// Continued by: Use where a standard work is being continued by somebody other than the original author
    A42,
    /// Interviewer
    A43,
    /// Interviewee
    A44,
    /// Comic script by: Writer of dialogue, captions in a comic book (following an outline by the primary writer)
    A45,
    /// Inker: Renders final comic book line art based on work of the illustrator or penciller. Preferred to code A40
    A46,
    /// Colorist: Provides comic book color art and effects. Preferred to code A40
    A47,
    /// Letterer: Creates comic book text balloons and other text elements (where this is a distinct role from script writer and/or illustrator)
    A48,
    /// Research by: Person or organization responsible for performing research on which the work is based. For use in ONIX 3.0 only
    A51,
    /// Other primary creator: Other type of primary creator not specified above
    A99,
    /// Edited by
    B01,
    /// Revised by
    B02,
    /// Retold by
    B03,
    /// Abridged by
    B04,
    /// Adapted by
    B05,
    /// Translated by
    B06,
    /// As told by
    B07,
    /// Translated with commentary by: This code applies where a translator has provided a commentary on issues relating to the translation. If the translator has also provided a commentary on the work itself, codes B06 and A21 should be used
    B08,
    /// Series edited by: Name of a series editor when the product belongs to a series
    B09,
    /// Edited and translated by
    B10,
    /// Editor-in-chief
    B11,
    /// Guest editor
    B12,
    /// Volume editor
    B13,
    /// Editorial board member
    B14,
    /// Editorial coordination by
    B15,
    /// Managing editor
    B16,
    /// Founded by: Usually the founder editor of a serial publication: Begruendet von
    B17,
    /// Prepared for publication by
    B18,
    /// Associate editor
    B19,
    /// Consultant editor: Use also for ‘advisory editor’, ‘series advisor’, ‘editorial consultant’ etc
    B20,
    /// General editor
    B21,
    /// Dramatized by
    B22,
    /// General rapporteur: In Europe, an expert editor who takes responsibility for the legal content of a collaborative law volume
    B23,
    /// Literary editor: An editor who is responsible for establishing the text used in an edition of a literary work, where this is recognised as a distinctive role (in Spain, ‘editor literario’)
    B24,
    /// Arranged by (music)
    B25,
    /// Technical editor    Responsible for the technical accuracy and language, may also be involved in coordinating and preparing technical material for publication  15: 30
    B26,
    /// Thesis advisor or supervisor
    B27,
    /// Thesis examiner
    B28,
    /// Scientific editor: Responsible overall for the scientific content of the publication
    B29,
    /// Historical advisor: For use in ONIX 3.0 only
    B30,
    /// Original editor: Editor of the first edition (usually of a standard work) who is not an editor of the current edition. For use in ONIX 3.0 only
    B31,
    /// Other adaptation by: Other type of adaptation or editing not specified above
    B99,
    /// Compiled by: For puzzles, directories, statistics, etc
    C01,
    /// Selected by: For textual material (eg for an anthology)
    C02,
    /// Non-text material selected by: Eg for a collection of photographs etc
    C03,
    /// Curated by: Eg for an exhibition
    C04,
    /// Other compilation by: Other type of compilation not specified above
    C99,
    /// Producer
    D01,
    /// Director
    D02,
    /// Conductor: Conductor of a musical performance
    D03,
    /// Choreographer: Of a dance performance. For use in ONIX 3.0 only
    D04,
    /// Other direction by: Other type of direction not specified above
    D99,
    /// Actor: Performer in a dramatized production (including a voice actor in an audio production)
    E01,
    /// Dancer
    E02,
    /// Narrator: Where the narrator is a character in a dramatized production (including a voice actor in an audio production). For the ‘narrator’ of a non-dramatized audiobook, see code E07
    E03,
    /// Commentator
    E04,
    /// Vocal soloist: Singer etc
    E05,
    /// Instrumental soloist
    E06,
    /// Read by: Reader of recorded text, as in an audiobook
    E07,
    /// Performed by (orchestra, band, ensemble): Name of a musical group in a performing role
    E08,
    /// Speaker: Of a speech, lecture etc
    E09,
    /// Presenter: Introduces and links other contributors and material, eg within a documentary
    E10,
    /// Performed by: Other type of performer not specified above: use for a recorded performance which does not fit a category above, eg a performance by a stand-up comedian
    E99,
    /// Filmed/photographed by: Cinematographer, etc
    F01,
    /// Editor (film or video)
    F02,
    /// Other recording by: Other type of recording not specified above
    F99,
    /// Assisted by: May be associated with any contributor role, and placement should therefore be controlled by contributor sequence numbering
    Z01,
    /// Honored/dedicated to
    Z02,
    /// (Various roles): For use ONLY with ‘et al’ or ‘Various’ within <UnnamedPersons>, where the roles of the multiple contributors vary
    Z98,
    /// Other   Other creative responsibility not falling within A to F above
    Z99,
}

impl FromStr for OnixContributorCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a01" | "A01" => Ok(OnixContributorCode::A01),
            "a02" | "A02" => Ok(OnixContributorCode::A02),
            "a03" | "A03" => Ok(OnixContributorCode::A03),
            "a04" | "A04" => Ok(OnixContributorCode::A04),
            "a05" | "A05" => Ok(OnixContributorCode::A05),
            "a06" | "A06" => Ok(OnixContributorCode::A06),
            "a07" | "A07" => Ok(OnixContributorCode::A07),
            "a08" | "A08" => Ok(OnixContributorCode::A08),
            "a09" | "A09" => Ok(OnixContributorCode::A09),
            "a10" | "A10" => Ok(OnixContributorCode::A10),
            "a11" | "A11" => Ok(OnixContributorCode::A11),
            "a12" | "A12" => Ok(OnixContributorCode::A12),
            "a13" | "A13" => Ok(OnixContributorCode::A13),
            "a14" | "A14" => Ok(OnixContributorCode::A14),
            "a15" | "A15" => Ok(OnixContributorCode::A15),
            "a16" | "A16" => Ok(OnixContributorCode::A16),
            "a17" | "A17" => Ok(OnixContributorCode::A17),
            "a18" | "A18" => Ok(OnixContributorCode::A18),
            "a19" | "A19" => Ok(OnixContributorCode::A19),
            "a20" | "A20" => Ok(OnixContributorCode::A20),
            "a21" | "A21" => Ok(OnixContributorCode::A21),
            "a22" | "A22" => Ok(OnixContributorCode::A22),
            "a23" | "A23" => Ok(OnixContributorCode::A23),
            "a24" | "A24" => Ok(OnixContributorCode::A24),
            "a25" | "A25" => Ok(OnixContributorCode::A25),
            "a26" | "A26" => Ok(OnixContributorCode::A26),
            "a27" | "A27" => Ok(OnixContributorCode::A27),
            "a29" | "A29" => Ok(OnixContributorCode::A29),
            "a30" | "A30" => Ok(OnixContributorCode::A30),
            "a31" | "A31" => Ok(OnixContributorCode::A31),
            "a32" | "A32" => Ok(OnixContributorCode::A32),
            "a33" | "A33" => Ok(OnixContributorCode::A33),
            "a34" | "A34" => Ok(OnixContributorCode::A34),
            "a35" | "A35" => Ok(OnixContributorCode::A35),
            "a36" | "A36" => Ok(OnixContributorCode::A36),
            "a37" | "A37" => Ok(OnixContributorCode::A37),
            "a38" | "A38" => Ok(OnixContributorCode::A38),
            "a39" | "A39" => Ok(OnixContributorCode::A39),
            "a40" | "A40" => Ok(OnixContributorCode::A40),
            "a41" | "A41" => Ok(OnixContributorCode::A41),
            "a42" | "A42" => Ok(OnixContributorCode::A42),
            "a43" | "A43" => Ok(OnixContributorCode::A43),
            "a44" | "A44" => Ok(OnixContributorCode::A44),
            "a45" | "A45" => Ok(OnixContributorCode::A45),
            "a46" | "A46" => Ok(OnixContributorCode::A46),
            "a47" | "A47" => Ok(OnixContributorCode::A47),
            "a48" | "A48" => Ok(OnixContributorCode::A48),
            "a51" | "A51" => Ok(OnixContributorCode::A51),
            "a99" | "A99" => Ok(OnixContributorCode::A99),
            "b01" | "B01" => Ok(OnixContributorCode::B01),
            "b02" | "B02" => Ok(OnixContributorCode::B02),
            "b03" | "B03" => Ok(OnixContributorCode::B03),
            "b04" | "B04" => Ok(OnixContributorCode::B04),
            "b05" | "B05" => Ok(OnixContributorCode::B05),
            "b06" | "B06" => Ok(OnixContributorCode::B06),
            "b07" | "B07" => Ok(OnixContributorCode::B07),
            "b08" | "B08" => Ok(OnixContributorCode::B08),
            "b09" | "B09" => Ok(OnixContributorCode::B09),
            "b10" | "B10" => Ok(OnixContributorCode::B10),
            "b11" | "B11" => Ok(OnixContributorCode::B11),
            "b12" | "B12" => Ok(OnixContributorCode::B12),
            "b13" | "B13" => Ok(OnixContributorCode::B13),
            "b14" | "B14" => Ok(OnixContributorCode::B14),
            "b15" | "B15" => Ok(OnixContributorCode::B15),
            "b16" | "B16" => Ok(OnixContributorCode::B16),
            "b17" | "B17" => Ok(OnixContributorCode::B17),
            "b18" | "B18" => Ok(OnixContributorCode::B18),
            "b19" | "B19" => Ok(OnixContributorCode::B19),
            "b20" | "B20" => Ok(OnixContributorCode::B20),
            "b21" | "B21" => Ok(OnixContributorCode::B21),
            "b22" | "B22" => Ok(OnixContributorCode::B22),
            "b23" | "B23" => Ok(OnixContributorCode::B23),
            "b24" | "B24" => Ok(OnixContributorCode::B24),
            "b25" | "B25" => Ok(OnixContributorCode::B25),
            "b26" | "B26" => Ok(OnixContributorCode::B26),
            "b27" | "B27" => Ok(OnixContributorCode::B27),
            "b28" | "B28" => Ok(OnixContributorCode::B28),
            "b29" | "B29" => Ok(OnixContributorCode::B29),
            "b30" | "B30" => Ok(OnixContributorCode::B30),
            "b31" | "B31" => Ok(OnixContributorCode::B31),
            "b99" | "B99" => Ok(OnixContributorCode::B99),
            "c01" | "C01" => Ok(OnixContributorCode::C01),
            "c02" | "C02" => Ok(OnixContributorCode::C02),
            "c03" | "C03" => Ok(OnixContributorCode::C03),
            "c04" | "C04" => Ok(OnixContributorCode::C04),
            "c99" | "C99" => Ok(OnixContributorCode::C99),
            "d01" | "D01" => Ok(OnixContributorCode::D01),
            "d02" | "D02" => Ok(OnixContributorCode::D02),
            "d03" | "D03" => Ok(OnixContributorCode::D03),
            "d04" | "D04" => Ok(OnixContributorCode::D04),
            "d99" | "D99" => Ok(OnixContributorCode::D99),
            "e01" | "E01" => Ok(OnixContributorCode::E01),
            "e02" | "E02" => Ok(OnixContributorCode::E02),
            "e03" | "E03" => Ok(OnixContributorCode::E03),
            "e04" | "E04" => Ok(OnixContributorCode::E04),
            "e05" | "E05" => Ok(OnixContributorCode::E05),
            "e06" | "E06" => Ok(OnixContributorCode::E06),
            "e07" | "E07" => Ok(OnixContributorCode::E07),
            "e08" | "E08" => Ok(OnixContributorCode::E08),
            "e09" | "E09" => Ok(OnixContributorCode::E09),
            "e10" | "E10" => Ok(OnixContributorCode::E10),
            "e99" | "E99" => Ok(OnixContributorCode::E99),
            "f01" | "F01" => Ok(OnixContributorCode::F01),
            "f02" | "F02" => Ok(OnixContributorCode::F02),
            "f99" | "F99" => Ok(OnixContributorCode::F99),
            "z01" | "Z01" => Ok(OnixContributorCode::Z01),
            "z02" | "Z02" => Ok(OnixContributorCode::Z02),
            "z98" | "Z98" => Ok(OnixContributorCode::Z98),
            "z99" | "Z99" => Ok(OnixContributorCode::Z99),
            _ => Err(()),
        }
    }
}

/// Onix codelist 153 (<https://ns.editeur.org/onix/en/153>)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Hash)]
pub enum OnixTextType {
    /// Sender-defined text: To be used only in circumstances where the parties to an exchange have agreed to include text which (a) is not for general distribution, and (b) cannot be coded elsewhere. If more than one type of text is sent, it must be identified by tagging within the text itself
    T01,
    /// Short description/annotation: Limited to a maximum of 350 characters
    T02,
    /// Description: Length unrestricted
    T03,
    /// Table of contents: Used for a table of contents sent as a single text field, which may or may not carry structure expressed using XHTML
    T04,
    /// Flap / cover copy: Primary descriptive blurb taken from the back cover and/or flaps. See also code 27
    T05,
    /// Review quote: A quote taken from a review of the product or of the work in question where there is no need to take account of different editions
    T06,
    /// Review quote: previous edition: A quote taken from a review of a previous edition of the work
    T07,
    /// Review quote: previous work: A quote taken from a review of a previous work by the same author(s) or in the same series
    T08,
    /// Endorsement: A quote usually provided by a celebrity or another author to promote a new book, not from a review
    T09,
    /// Promotional headline: A promotional phrase which is intended to headline a description of the product
    T10,
    /// Feature: Text describing a feature of a product to which the publisher wishes to draw attention for promotional purposes. Each separate feature should be described by a separate repeat, so that formatting can be applied at the discretion of the receiver of the ONIX record, or multiple features can be described using appropriate XHTML markup
    T11,
    /// Biographical note: A note referring to all contributors to a product – NOT linked to a single contributor
    T12,
    /// Publisher’s notice: A statement included by a publisher in fulfillment of contractual obligations, such as a disclaimer, sponsor statement, or legal notice of any sort. Note that the inclusion of such a notice cannot and does not imply that a user of the ONIX record is obliged to reproduce it
    T13,
    /// Excerpt: A short excerpt from the main text of the work
    T14,
    /// Index: Used for an index sent as a single text field, which may be structured using XHTML
    T15,
    /// Short description/annotation for collection: (of which the product is a part.) Limited to a maximum of 350 characters
    T16,
    /// Description for collection: (of which the product is a part.) Length unrestricted
    T17,
    /// New feature: As code 11 but used for a new feature of this edition or version
    T18,
    /// Version history
    T19,
    /// Open access statement: Short summary statement of open access status and any related conditions (eg ‘Open access – no commercial use’), primarily for marketing purposes. Should always be accompanied by a link to the complete license (see <EpubLicense> or code 99 in List 158)
    T20,
    /// Digital exclusivity statement: Short summary statement that the product is available only in digital formats (eg ‘Digital exclusive’). If a non-digital version is planned, <ContentDate> should be used to specify the date when exclusivity will end (use content date role code 15). If a non-digital version is available, the statement should not be included
    T21,
    /// Official recommendation: For example a recommendation or approval provided by a ministry of education or other official body. Use <Text> to provide details and ideally use <TextSourceCorporate> to name the approver
    T22,
    /// JBPA description: Short description in format specified by Japanese Book Publishers Association
    T23,
    /// schema.org snippet: JSON-LD snippet suitable for use within an HTML <script type="application/ld+json"> tag, containing structured metadata suitable for use with schema.org
    T24,
    /// Errata
    T25,
    /// Introduction: Introduction, preface or the text of other preliminary material, sent as a single text field, which may be structured using XHTML
    T26,
    /// Secondary flap / cover copy: Secondary descriptive blurb taken from the back cover and/or flaps, used only when there are two separate texts and the primary text is included using code 05
    T27,
    /// Full cast and credit list: For use with dramatized audiobooks, filmed entertainment etc, for a cast list sent as a single text field, which may or may not carry structure expressed using XHTML
    T28,
    /// Bibliography: Complete list of books by the author(s), supplied as a single text field, which may be structured using (X)HTML
    T29,
    /// Abstract: Formal summary of content (normally used with academic and scholarly content only)
    T30,
    /// Rules or instructions: Eg for a game, kit
    T31,
}

/// Try to map a code from one scheme to another.
///
/// The primary purpose is to shift from a nuanced encoding scheme,
/// onix, to one which may be less flexible but is more likely to be natively recognised
/// by reading systems.
pub trait ValueMapping<T> {
    /// map this code to one of `T` if possible
    fn map_code(&self) -> Option<T>;
}

impl ValueMapping<EpubTitleType> for OnixTitleCode {
    fn map_code(&self) -> Option<EpubTitleType> {
        match self {
            OnixTitleCode::T05 => Some(EpubTitleType::Short),
            OnixTitleCode::T13 => Some(EpubTitleType::Expanded),
            OnixTitleCode::T01 => Some(EpubTitleType::Main),
            _ => None,
        }
    }
}

impl ValueMapping<MarcRelator> for OnixContributorCode {
    fn map_code(&self) -> Option<MarcRelator> {
        use MarcRelator::*;
        /// Taken from <http://www.oclc.org/research/publications/library/2012/2012-04a.xls>
        use OnixContributorCode::*;
        match self {
            A01 => Some(Aut),
            A02 => Some(Ctb),
            A03 => Some(Aus),
            A04 => Some(Lbt),
            A05 => Some(Lyr),
            A06 => Some(Cmp),
            A07 => Some(Art),
            A08 => Some(Pht),
            A09 => Some(Cre),
            A10 => Some(Cre),
            A11 => Some(Dsr),
            A12 => Some(Ill),
            A13 => Some(Pht),
            A14 => Some(Ctb),
            A15 => Some(Aui),
            A16 => Some(Aui),
            A17 => Some(Ctb),
            A18 => Some(Ctb),
            A19 => Some(Aft),
            A20 => Some(Ctb),
            A21 => Some(Cwt),
            A22 => Some(Aft),
            A23 => Some(Aui),
            A24 => Some(Aui),
            A25 => Some(Ctb),
            A26 => None,
            A27 => Some(Ctb),
            A29 => Some(Aui),
            A30 => Some(Prg),
            A31 => Some(Lyr),
            A32 => Some(Ctb),
            A33 => None,
            A34 => Some(Ctb),
            A35 => Some(Ill),
            A36 => Some(Cov),
            A37 => Some(Ant),
            A38 => Some(Aut),
            A39 => Some(Ctg),
            A40 => Some(Ill),
            A41 => None,
            A42 => None,
            A43 => Some(Ivr),
            A44 => Some(Ive),
            A99 => Some(Cre),
            B01 => Some(Edt),
            B02 => Some(Edt),
            B03 => Some(Nrt),
            B04 => Some(Edt),
            B05 => Some(Adp),
            B06 => Some(Trl),
            B07 => Some(Nrt),
            B08 => Some(Trl),
            B09 => Some(Edt),
            B10 => Some(Trl),
            B11 => Some(Pbd),
            B12 => Some(Edt),
            B13 => Some(Edt),
            B14 => Some(Edt),
            B15 => Some(Edt),
            B16 => Some(Edt),
            B17 => None,
            B18 => Some(Pbl),
            B19 => Some(Edt),
            B20 => Some(Edt),
            B21 => Some(Edt),
            B22 => None,
            B23 => Some(Edt),
            B24 => Some(Edt),
            B25 => Some(Arr),
            B99 => Some(Adp),
            C01 => Some(Com),
            C02 => Some(Com),
            C99 => Some(Com),
            D01 => Some(Pro),
            D02 => Some(Drt),
            D03 => Some(Cnd),
            D99 => Some(Drt),
            E01 => Some(Act),
            E02 => Some(Dnc),
            E03 => Some(Nrt),
            E04 => Some(Cmm),
            E05 => Some(Sng),
            E06 => Some(Itr),
            E07 => Some(Nrt),
            E08 => Some(Prf),
            E99 => Some(Prf),
            F01 => Some(Pht),
            F99 => None,
            Z01 => None,
            Z99 => None,
            _ => None,
        }
    }
}
